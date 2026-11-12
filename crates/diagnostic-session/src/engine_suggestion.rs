//! From SDD's own words for an engine, as its VIN tables give them
//! ("508PN - V8 5.0 NA Petrol"), to the engine code the same data qualifies
//! its records by ("V8NA") — only where the words point at exactly one of the
//! programme's codes. The person confirms the choice on the vehicle card; a
//! text that fits two codes, or none, suggests nothing. Asked for by the
//! first tester, whose Discovery 4 decoded to a V8 the card then left
//! "not stated" (2026-11-12).

/// The one engine code of `powertrains` the decoded engine text fits, if
/// exactly one does.
pub fn suggest_powertrain(powertrains: &[String], engine_text: &str) -> Option<String> {
    let tokens = tokens(engine_text);
    let mut fitting = powertrains.iter().filter(|code| code_fits(code, &tokens));
    let first = fitting.next()?;
    if fitting.next().is_some() {
        return None;
    }
    Some(first.clone())
}

/// The one variant of `variants` the text's displacement names, if the text
/// names a displacement and the data has that variant.
pub fn suggest_variant(variants: &[String], engine_text: &str) -> Option<String> {
    let litres = tokens(engine_text).into_iter().find_map(|token| {
        let number: f32 = token.trim_end_matches('L').parse().ok()?;
        (0.5..10.0).contains(&number).then_some(number)
    })?;
    let wanted = variant_name(litres);
    variants
        .iter()
        .find(|variant| variant.eq_ignore_ascii_case(&wanted))
        .cloned()
}

/// SDD writes a displacement as `5L`, `2.7L`, `3L`: whole litres without a
/// decimal, otherwise one decimal.
fn variant_name(litres: f32) -> String {
    if (litres - litres.round()).abs() < 0.01 {
        format!("{}L", litres.round() as u32)
    } else {
        format!("{litres:.1}L")
    }
}

/// The words of the text, upper case, with "S/C" read as one word.
fn tokens(text: &str) -> Vec<String> {
    text.to_ascii_uppercase()
        .replace("S/C", "SC")
        .split(|character: char| !character.is_ascii_alphanumeric() && character != '.')
        .filter(|token| !token.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Whether one engine code is what the words describe. A code names its
/// cylinders and then its kind (`V8` + `NA`, `V6` + `DIESEL`, `I4` + `D`,
/// `I6` + `PETROL`); a code without a cylinder prefix (`GTDI`, `HYBRID`,
/// `TDCI`) must appear in the words as it is.
fn code_fits(code: &str, tokens: &[String]) -> bool {
    let code = code.to_ascii_uppercase();
    let has = |word: &str| tokens.iter().any(|token| token == word);
    let Some((cylinders, kind)) = split_cylinders(&code) else {
        return has(&code);
    };
    if !has(cylinders) {
        return false;
    }
    let diesel = has("DIESEL");
    let supercharged = has("SC") || has("SUPERCHARGED");
    let petrol = has("PETROL") || has("P");
    match kind {
        "DIESEL" | "D" => diesel,
        "NA" => has("NA") || (petrol && !supercharged && !diesel),
        "SC" => supercharged,
        "PETROL" => petrol && !diesel,
        _ => false,
    }
}

fn split_cylinders(code: &str) -> Option<(&'static str, &str)> {
    ["STR6", "V6", "V8", "I4", "I6"]
        .into_iter()
        .find_map(|prefix| code.strip_prefix(prefix).map(|kind| (prefix, kind)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn codes(list: &[&str]) -> Vec<String> {
        list.iter().map(|code| (*code).to_owned()).collect()
    }

    #[test]
    fn the_discovery_4_engines_are_told_apart_by_sdds_own_words() {
        let l319 = codes(&["V6DIESEL", "V8NA", "V6NA", "V6SC"]);
        assert_eq!(
            suggest_powertrain(&l319, "508PN - V8 5.0 NA Petrol").as_deref(),
            Some("V8NA")
        );
        assert_eq!(
            suggest_powertrain(&l319, "306DT - V6 3.0 Hi Pwr Diesel with cDPF").as_deref(),
            Some("V6DIESEL")
        );
        assert_eq!(
            suggest_powertrain(&l319, "306PS - V6 3.0 SC Petrol").as_deref(),
            Some("V6SC")
        );
        assert_eq!(
            suggest_powertrain(&l319, "406PN - V6 4.0 Petrol").as_deref(),
            Some("V6NA")
        );
    }

    #[test]
    fn words_that_fit_two_codes_or_none_suggest_nothing() {
        let x250 = codes(&[
            "V6NA", "V6DIESEL", "V8SC", "V8NA", "I4DIESEL", "V6SC", "GTDI",
        ]);
        // Two engines in one table cell, and no cylinder count among the words.
        assert_eq!(
            suggest_powertrain(&x250, "5.0L AJ-133 or AJ126 3.0L SC (P)"),
            None
        );
        // A four-cylinder diesel named by its family, not by its fuel.
        assert_eq!(suggest_powertrain(&x250, "2.0L AJD-I4"), None);
        // Two codes for the same words.
        let both = codes(&["I4D", "I4DIESEL"]);
        assert_eq!(suggest_powertrain(&both, "204DT - I4 2.0 Diesel"), None);
        // A code the data never qualifies by.
        assert_eq!(
            suggest_powertrain(&codes(&["SYNTHENGINE"]), "3.0L V6 synthetic"),
            None
        );
        assert_eq!(suggest_powertrain(&[], "508PN - V8 5.0 NA Petrol"), None);
    }

    #[test]
    fn a_code_without_a_cylinder_count_must_be_in_the_words_as_it_is() {
        let evoque = codes(&["I4DIESEL", "GTDI"]);
        assert_eq!(
            suggest_powertrain(&evoque, "204PT - 2.0 GTDi Petrol").as_deref(),
            Some("GTDI")
        );
        assert_eq!(
            suggest_powertrain(&evoque, "DW12 - 2.2 DIESEL (Low power)"),
            None
        );
    }

    #[test]
    fn the_variant_is_the_displacement_the_data_splits_by() {
        let diesel = codes(&["2.7L", "3L"]);
        assert_eq!(
            suggest_variant(&diesel, "306DT - V6 3.0 Hi Pwr Diesel with cDPF").as_deref(),
            Some("3L")
        );
        assert_eq!(
            suggest_variant(&diesel, "276DT - V6 2.7 Diesel non cDPF").as_deref(),
            Some("2.7L")
        );
        assert_eq!(
            suggest_variant(&codes(&["4.2L", "5L"]), "5.0L AJ-133 or AJ126 3.0L SC (P)").as_deref(),
            Some("5L")
        );
        // The V8 of the Discovery 4 is not split: nothing to suggest.
        assert_eq!(suggest_variant(&[], "508PN - V8 5.0 NA Petrol"), None);
        // A number that is not a displacement is not read as one.
        assert_eq!(
            suggest_variant(&codes(&["5L"]), "508PN - V8 NA Petrol"),
            None
        );
    }
}
