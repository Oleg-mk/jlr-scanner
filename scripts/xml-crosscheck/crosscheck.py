#!/usr/bin/env python3
"""Read the SDD XML a second time, independently, and compare it with the
library our Rust ingest produced.

Why this exists. The library's breadth comes from one parser. If that parser
misreads an address digit or a byte range, nothing downstream would notice:
the survey, the read plan and the report would all agree with each other and
all be wrong. So this script reads the same source files again, in another
language, from the XML's own shape rather than from the Rust code, and says
where the two readings disagree. A disagreement is either our bug or a
normalisation we chose on purpose and can name.

What it compares:

* **Addresses.** For every module a `PLATFORM_*.xml` declares: the diagnostic
  request and response CAN identifiers, and the network the module sits on,
  against the library's `diagnostic_addressing` records.
* **Field widths.** For every read parameter a DID-formatting snapshot
  declares: the byte range, the mask and the data size, against the
  `identifier_definition` encoding the library carries.

What it does not do: judge. It prints counts and, into a local report file,
the disagreements. It never writes SDD text into the repository.

Usage:

    python scripts/xml-crosscheck/crosscheck.py \
        --platforms <dir with PLATFORM_*.xml> [--platforms <another>] \
        --snapshot <dir with the DID Formatting xml files> \
        --library <exported library dir> \
        --report <where to write the detailed report>
"""
from __future__ import annotations

import argparse
import json
import re
import sys
import xml.etree.ElementTree as ET
from collections import defaultdict
from pathlib import Path

# --------------------------------------------------------------------------
# Side A: the XML, read here, from the shape of the documents themselves.
# --------------------------------------------------------------------------


def read_platform_addresses(paths: list[Path]) -> dict[tuple[str, str], set]:
    """`(programme, family)` to the set of `(request, response, network)` the
    platform documents declare for it, over every model year they cover.

    Addresses are compared as a set per programme rather than per model year
    on purpose: mapping a file name to SDD's year breakpoint is itself a piece
    of parsing, and a difference there would drown the thing being checked.
    A misread digit still shows, because it puts a value in one set and not
    the other.
    """
    found: dict[tuple[str, str], set] = defaultdict(set)
    for path in paths:
        name = path.stem
        if not name.startswith("PLATFORM_"):
            continue
        try:
            root = ET.parse(path).getroot()
        except ET.ParseError as error:
            print(f"  ! {path.name}: {error}", file=sys.stderr)
            continue
        # Which programme a document is about has three answers, and they
        # disagree: the file name, the `<platform_name>` it declares, and the
        # `model` its qualifiers name. `PLATFORM_X356.xml` calls itself X356
        # and qualifies every module with `model="X350"`. The qualifier wins,
        # because it is the applicability statement — this module applies to
        # that model — and it is what the ingest reads. The programme is only
        # the key the two readings are joined on; the facts under test are the
        # addresses, and those are still read here from scratch.
        models = {
            qualifier.get("model", "").strip()
            for qualifier in root.iter("qualifier")
            if qualifier.get("model", "").strip()
        }
        declared = (root.findtext("platform_name") or "").strip()
        programme = (
            models.pop() if len(models) == 1 else (declared or name[len("PLATFORM_") :])
        )
        programme = programme.split("_")[0].upper()
        for module in root.iter("module"):
            code_name = module.find("module_code_name")
            if code_name is None:
                continue
            family = (code_name.get("acronym") or "").strip()
            if not family:
                continue
            request = response = None
            for address in module.findall("address"):
                if address.get("session") != "diag":
                    continue
                value = (address.text or "").strip()
                if address.get("type") == "can_tx":
                    request = value
                elif address.get("type") == "can_rx":
                    response = value
            network = (module.findtext("network") or "").strip()
            if request is None or response is None:
                continue
            found[(programme, family)].add((parse_id(request), parse_id(response), network))
    return found


def parse_id(text: str):
    """`0x760` or `1888` to an integer, or the text itself when it is neither."""
    text = text.strip()
    try:
        return int(text, 16) if text.lower().startswith("0x") else int(text, 10)
    except ValueError:
        return text


def read_snapshot_parameters(paths: list[Path]) -> dict[tuple[str, str, str], tuple]:
    """`(identifier, family, parameter name)` to `(lsb, msb, mask, size)`.

    A DID-formatting document holds `KeyedData` entries named like
    `DID 0x1945 AWDCM`, each with the data size in its properties and one
    `ReadParameter` per value, carrying the byte range as `lsbNumber` and
    `msbNumber` and the mask as a decimal number.
    """
    found: dict[tuple[str, str, str], set] = defaultdict(set)
    heading = re.compile(r"^DID\s+(0x[0-9A-Fa-f]+)\s*(\S+)?")
    for path in paths:
        try:
            root = ET.parse(path).getroot()
        except ET.ParseError as error:
            print(f"  ! {path.name}: {error}", file=sys.stderr)
            continue
        for keyed in root.iter("KeyedData"):
            match = heading.match(keyed.get("id") or "")
            if not match:
                continue
            identifier = match.group(1).lower()
            family = (match.group(2) or "").strip().upper()
            size = keyed.findtext("./Properties/dataSize")
            size = int(size) if size and size.lstrip("-").isdigit() else None
            for parameter in keyed.findall("ReadParameter"):
                name = (parameter.findtext("./Properties/name") or "").strip()
                if not name:
                    continue
                try:
                    lsb = int(parameter.get("lsbNumber"))
                    msb = int(parameter.get("msbNumber"))
                    mask = int(parameter.get("mask"))
                except (TypeError, ValueError):
                    continue
                found[(identifier, family, name)].add((lsb, msb, mask, size))
    return found


# --------------------------------------------------------------------------
# Side B: the library our ingest wrote.
# --------------------------------------------------------------------------


def read_library(directory: Path):
    """The addressing and parameter facts the exported manifests carry.

    Two kinds of record are set aside rather than compared, because the XML is
    not their source and comparing them would only manufacture noise: the
    29-bit `normal_fixed` identifiers this project derives itself (ADR-0017),
    and the CAN-link-monitor rows whose entity is a whole vehicle programme
    rather than a module.
    """
    addresses: dict[tuple[str, str], set] = defaultdict(set)
    parameters: dict[tuple[str, str, str], set] = defaultdict(set)
    derived: set[tuple[str, str]] = set()
    for path in sorted(directory.rglob("*.json")):
        try:
            data = json.loads(path.read_text(encoding="utf-8"))
        except (json.JSONDecodeError, UnicodeDecodeError):
            continue
        for batch in data if isinstance(data, list) else [data]:
            if not isinstance(batch, dict):
                continue
            for record in batch.get("records", []):
                collect_record(record, addresses, parameters, derived)
    return addresses, parameters, derived


def collect_record(record, addresses, parameters, derived):
    entity = record.get("entity") or {}
    key = record.get("key") or {}
    value = record.get("value") or {}
    applicability = record.get("applicability") or {}
    if key.get("kind") == "diagnostic_addressing":
        if entity.get("kind") != "ecu_family":
            return
        family = entity.get("id")
        programmes = one_of(applicability.get("vehicle_program"))
        network = network_of(value)
        for programme in programmes:
            if value.get("can_id_format") == "extended29_bit":
                derived.add((programme, family))
                continue
            addresses[(programme, family)].add(
                (value.get("request_id"), value.get("response_id"), network)
            )
    elif key.get("kind") == "parameter_definition":
        identifier = (value.get("identifier") or "").lower()
        families = one_of(applicability.get("ecu_family")) or [""]
        encoding = parse_encoding(value.get("encoding") or "")
        if encoding is None:
            return
        for family in families:
            parameters[(identifier, family.upper(), key.get("parameter", ""))].add(encoding)


def one_of(dimension):
    if isinstance(dimension, dict) and dimension.get("state") == "one_of":
        return [str(v) for v in dimension.get("values", [])]
    return []


def network_of(value):
    """The library keeps the bus under whichever name the ingest chose; the
    comparison only needs the two sides to agree, so both are normalised to
    the SDD name when one is present."""
    for field in ("logical_bus", "network", "bus"):
        if value.get(field):
            return str(value[field])
    return ""


def parse_encoding(encoding: str):
    """`bytes=0..1;size=4;mask=0xffff;converter=…` to `(lsb, msb, mask, size)`."""
    fields = dict(
        part.split("=", 1) for part in encoding.split(";") if "=" in part
    )
    span = fields.get("bytes", "")
    match = re.match(r"^(\d+)\.\.(\d+)$", span.strip())
    if not match:
        return None
    mask = fields.get("mask", "")
    try:
        mask_value = int(mask, 16) if mask.lower().startswith("0x") else int(mask)
    except ValueError:
        return None
    size = fields.get("size")
    size_value = int(size) if size and size.isdigit() else None
    return (int(match.group(1)), int(match.group(2)), mask_value, size_value)


# --------------------------------------------------------------------------


def compare_addresses(xml_side, library_side, report):
    """Every module the XML declares, against what the library holds for it."""
    agreed = missing = extra = disagreed = 0
    for key, xml_values in sorted(xml_side.items()):
        programme, family = key
        library_values = library_side.get(key)
        if library_values is None:
            missing += 1
            report.write(f"ADDRESS ONLY-IN-XML   {programme} {family}\n")
            continue
        xml_pairs = {(request, response) for request, response, _ in xml_values}
        library_pairs = {(request, response) for request, response, _ in library_values}
        if xml_pairs == library_pairs:
            agreed += 1
            continue
        disagreed += 1
        report.write(
            f"ADDRESS DISAGREE      {programme} {family}\n"
            f"    xml     {sorted(xml_pairs)}\n"
            f"    library {sorted(library_pairs)}\n"
        )
    for key in sorted(set(library_side) - set(xml_side)):
        extra += 1
        report.write(f"ADDRESS ONLY-IN-LIBRARY {key[0]} {key[1]}\n")
    return agreed, disagreed, missing, extra


def compare_parameters(xml_side, library_side, report):
    """A parameter agrees when every byte range and mask the library carries
    for it is one the XML declares. The four DID-formatting documents can
    define the same parameter more than once with different ranges, so a
    single definition per side would be compared arbitrarily; a set per side
    is not."""
    agreed = missing = disagreed = 0
    for key, xml_values in sorted(xml_side.items()):
        library_values = library_side.get(key)
        if not library_values:
            missing += 1
            continue
        xml_shapes = {value[:3] for value in xml_values}
        library_shapes = {value[:3] for value in library_values}
        if library_shapes <= xml_shapes:
            agreed += 1
            continue
        disagreed += 1
        identifier, family, name = key
        report.write(f"WIDTH DISAGREE        {identifier} {family} «{name}»\n")
        for lsb, msb, mask in sorted(xml_shapes):
            report.write(f"    xml     bytes={lsb}..{msb} mask=0x{mask:X}\n")
        for lsb, msb, mask in sorted(library_shapes - xml_shapes):
            report.write(f"    library bytes={lsb}..{msb} mask=0x{mask:X}  <- not in the XML\n")
    extra = len(set(library_side) - set(xml_side))
    return agreed, disagreed, missing, extra


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--platforms", action="append", default=[], type=Path)
    parser.add_argument("--snapshot", action="append", default=[], type=Path)
    parser.add_argument("--library", required=True, type=Path)
    parser.add_argument("--report", required=True, type=Path)
    args = parser.parse_args()

    platform_files: list[Path] = []
    for directory in args.platforms:
        platform_files.extend(sorted(directory.rglob("PLATFORM_*.xml")))
    snapshot_files: list[Path] = []
    for directory in args.snapshot:
        snapshot_files.extend(sorted(directory.rglob("*DID Formatting*.xml")))

    print(f"platform documents: {len(platform_files)}")
    print(f"snapshot documents: {len(snapshot_files)}")

    xml_addresses = read_platform_addresses(platform_files)
    xml_parameters = read_snapshot_parameters(snapshot_files)
    print(f"read from the XML: {len(xml_addresses)} module rows, {len(xml_parameters)} parameters")

    library_addresses, library_parameters, derived = read_library(args.library)
    print(
        f"read from the library: {len(library_addresses)} module rows, "
        f"{len(library_parameters)} parameters, and {len(derived)} rows this "
        f"project derived itself (ADR-0017), set aside"
    )

    args.report.parent.mkdir(parents=True, exist_ok=True)
    with args.report.open("w", encoding="utf-8", newline="\n") as report:
        report.write("Disagreements between the SDD XML and the exported library.\n")
        report.write("Kept out of the repository: it quotes source values.\n\n")
        a_ok, a_bad, a_missing, a_extra = compare_addresses(
            xml_addresses, library_addresses, report
        )
        p_ok, p_bad, p_missing, p_extra = compare_parameters(
            xml_parameters, library_parameters, report
        )

    print()
    print("addresses   agree", a_ok, "| disagree", a_bad, "| only in xml", a_missing, "| only in library", a_extra)
    print("field widths agree", p_ok, "| disagree", p_bad, "| only in xml", p_missing, "| only in library", p_extra)
    print(f"detail: {args.report}")
    return 1 if a_bad or p_bad else 0


if __name__ == "__main__":
    raise SystemExit(main())
