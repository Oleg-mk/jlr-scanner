//! Which parameters are a car's mileage, and which only look like it
//! (ADR-0024).
//!
//! A car keeps its distance in dozens of modules, each counting for its own
//! reasons, and SDD names those parameters. Two kinds matter and one must be
//! kept out:
//!
//! * **Current** — the module's own running total. `Total distance` is
//!   declared for ninety-two module families; the instrument cluster adds
//!   `Odometer store`.
//! * **Event** — the odometer as it stood when something was recorded: a
//!   gearbox stall, a failed gear selection, a delayed park engagement, a
//!   CAN-signal timeout, the flight recorder. An incident stamped at a
//!   mileage the car has not reached is the sharpest reading of all.
//! * **Neither** — `Distance travelled since the malfunction indicator lamp
//!   was activated` and its kin. Those are legislated counters that reset
//!   with the codes; they say nothing about a car's life, and showing one as
//!   an odometer would earn every doubt that followed.
//!
//! The rule lives here, beside the catalogue that names the parameters, and
//! not in the shell that draws the table.

/// What a mileage-shaped parameter actually is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MileageKind {
    /// The module's own running total.
    Current,
    /// The odometer as it stood when something was recorded.
    Event,
}

/// Whether SDD's name for a parameter is a mileage, and of which kind.
pub fn mileage_kind(parameter: &str) -> Option<MileageKind> {
    let name = parameter.to_ascii_lowercase();
    // Excluded by name rather than by pattern: these are the legislated
    // counters, and they reset.
    if name.contains("since")
        || name.contains("trip")
        || name.contains("malfunction indicator")
        || name.contains("mil ")
    {
        return None;
    }
    let trimmed = name.trim();
    if trimmed == "total distance" || trimmed == "odometer store" || trimmed == "odometer" {
        return Some(MileageKind::Current);
    }
    // Anything else that names an odometer, or carries a total distance
    // inside a larger record, is a stamp from a moment in the car's life.
    if name.contains("odomet") || name.contains("total distance") {
        return Some(MileageKind::Event);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_running_total_is_told_apart_from_a_stamp_and_from_a_counter() {
        // What ninety-two modules carry, and what the cluster adds.
        assert_eq!(mileage_kind("Total distance"), Some(MileageKind::Current));
        assert_eq!(mileage_kind("Odometer store"), Some(MileageKind::Current));

        // The odometer as it stood when something happened.
        assert_eq!(
            mileage_kind(
                "Gearbox stall event history  -  Odometer reading of last near stall event"
            ),
            Some(MileageKind::Event)
        );
        assert_eq!(
            mileage_kind("First CAN signal timeout event log  -  Total distance"),
            Some(MileageKind::Event)
        );
        assert_eq!(
            mileage_kind("Revised Jaguar flight recorder data  -  Odometer"),
            Some(MileageKind::Event)
        );
        assert_eq!(
            mileage_kind("Odometer reading of last failed drive gear selection"),
            Some(MileageKind::Event)
        );

        // The legislated counters, which reset with the codes.
        assert_eq!(
            mileage_kind("Distance travelled since the malfunction indicator lamp was activated"),
            None
        );
        assert_eq!(
            mileage_kind("Distance travelled while malfunction indicator activated"),
            None
        );
        assert_eq!(mileage_kind("Distance since codes cleared"), None);
        assert_eq!(mileage_kind("Trip distance"), None);

        // Everything else a module reads.
        assert_eq!(mileage_kind("Engine coolant temperature"), None);
        assert_eq!(mileage_kind("Vehicle speed"), None);
        assert_eq!(mileage_kind("Distance to empty"), None);
    }
}
