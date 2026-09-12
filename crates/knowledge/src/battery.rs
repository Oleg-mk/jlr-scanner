//! Which identifiers are the car's battery, and what each one is for
//! (ADR-0030).
//!
//! SDD's platform documents name, per module and per car, the identifiers
//! the battery monitor answers: how full the battery is, what it is doing
//! now, what it leaks while the car is parked, how it has aged, and what the
//! car remembers about it. They are ordinary `0x22` identifiers among
//! hundreds of others, so something has to say which of them are the
//! battery — and that something is a rule, not a word match: `Turbocharger
//! valve offset values` contains "charge" and is not a battery parameter.
//!
//! The rule is therefore a curated table of identifiers with the role each
//! plays, and a name check on top of it: an identifier is a battery
//! parameter only when the table holds it **and** the name the document
//! gives it on that module names the battery or the charging system. Neither
//! test alone is enough — the table alone would trust a number that means
//! something else on another module, and the name alone would let the
//! turbocharger in.
//!
//! The rule lives here, beside the records it classifies, and not in the
//! shell that draws the card. The precedent is `ADR-0024`'s mileage rule.

/// What a battery parameter is for — the group it belongs to on the card.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum BatteryRole {
    /// How full it is: state of charge, now and at its lowest.
    Charge,
    /// A voltage: the battery's own, or the generator's set point.
    Voltage,
    /// Current flowing in or out right now.
    Current,
    /// The battery's own temperature, as the monitor estimates it.
    Temperature,
    /// What the car draws while it is parked, and the relay box that
    /// governs it.
    Drain,
    /// How it has aged: cold cranking voltage, amp-hour loss, fault
    /// counters. Never a health percentage — SDD keeps none.
    Health,
    /// What the car remembers: time in service, resets, cumulative charge
    /// and discharge, statistics, the odometer at the last shutdowns.
    History,
    /// What the battery is declared to be: type, and the settings that
    /// depend on it.
    Configuration,
    /// The traction battery of a hybrid, from its own module.
    Hybrid,
}

impl BatteryRole {
    /// A stable machine name; the interface words it in the user's language.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Charge => "CHARGE",
            Self::Voltage => "VOLTAGE",
            Self::Current => "CURRENT",
            Self::Temperature => "TEMPERATURE",
            Self::Drain => "DRAIN",
            Self::Health => "HEALTH",
            Self::History => "HISTORY",
            Self::Configuration => "CONFIGURATION",
            Self::Hybrid => "HYBRID",
        }
    }

    /// The order the groups are shown in: what a person asks first, first.
    pub const fn order(self) -> u8 {
        match self {
            Self::Charge => 0,
            Self::Voltage => 1,
            Self::Current => 2,
            Self::Temperature => 3,
            Self::Drain => 4,
            Self::Health => 5,
            Self::History => 6,
            Self::Configuration => 7,
            Self::Hybrid => 8,
        }
    }
}

/// The curated table: every identifier this product treats as the battery,
/// with the role it plays and whether it belongs on the card's face.
/// Measured from the 45 platform documents on 2026-09-12 (ADR-0030).
const BATTERY_IDENTIFIERS: &[(u16, BatteryRole, bool)] = &[
    // How full it is.
    (0x4028, BatteryRole::Charge, true), // estimated state of charge
    (0x4035, BatteryRole::Charge, false), // lowest estimated state of charge
    (0x41C3, BatteryRole::Charge, false), // lowest state of charge
    (0x41CD, BatteryRole::Charge, false), // state of charge, battery 2
    // Voltage and current.
    (0x402A, BatteryRole::Voltage, true), // vehicle battery voltage
    (0x0304, BatteryRole::Voltage, false), // generator/alternator set point
    (0x4090, BatteryRole::Current, true), // battery current
    (0x402B, BatteryRole::Current, true), // battery current, earlier cars
    (0x4029, BatteryRole::Temperature, true), // estimated temperature
    // What it leaks while parked.
    (0x4025, BatteryRole::Drain, true), // quiescent current, low range / 24 h
    (0x401D, BatteryRole::Drain, false), // quiescent current, previous 15 min
    (0x41D0, BatteryRole::Drain, false), // quiescent current, high range
    (0x40D7, BatteryRole::Drain, false), // quiescent current statistics
    (0x41E6, BatteryRole::Drain, false), // quiescent current statistics
    (0xEEBB, BatteryRole::Drain, false), // quiescent current statistics
    (0x41EF, BatteryRole::Drain, false), // relay box, relay 1 open events
    (0x41F3, BatteryRole::Drain, false), // relay box, relay 2 open events
    (0x41F4, BatteryRole::Drain, false), // relay box, relay 3 open events
    (0x41F5, BatteryRole::Drain, false), // relay box, relay 4 open events
    (0x41F6, BatteryRole::Drain, false), // relay box fault status
    (0x41F7, BatteryRole::Drain, false), // relay box recent events store
    (0x41F8, BatteryRole::Drain, false), // relay box status measured
    (0x41FB, BatteryRole::Drain, false), // relay box relay target status
    // How it has aged.
    (0x41EA, BatteryRole::Health, true), // cold cranking voltage at present SoC
    (0x402E, BatteryRole::Health, false), // amp-hour charge loss
    (0x4047, BatteryRole::Health, false), // lowest calculated amp-hour value
    (0x40FC, BatteryRole::Health, false), // monitor sensor fault statistics
    (0x414D, BatteryRole::Health, false), // alternator fault counters
    (0x41DA, BatteryRole::Health, false), // dual battery system fault information
    // What the car remembers.
    (0x4027, BatteryRole::History, true), // time in service, days
    (0x4020, BatteryRole::History, true), // battery replacement / monitor resets
    (0x401E, BatteryRole::History, false), // cumulative charge, ignition on
    (0x4021, BatteryRole::History, false), // cumulative discharge, ignition on
    (0x4026, BatteryRole::History, false), // cumulative discharge, ignition off
    (0x401C, BatteryRole::History, false), // cumulative discharge after shutdown
    (0x409E, BatteryRole::History, false), // normalised cumulative discharge, total
    (0x422C, BatteryRole::History, false), // odometer at last five shutdowns
    (0x41E4, BatteryRole::History, false), // charge balance statistics
    (0x40A2, BatteryRole::History, false), // charge balance statistics counter
    (0x41E5, BatteryRole::History, false), // state of charge statistics
    (0x4094, BatteryRole::History, false), // state of charge statistics counter
    (0xEEB0, BatteryRole::History, false), // state of charge statistics
    (0xEEB1, BatteryRole::History, false), // charge statistics
    (0x414B, BatteryRole::History, false), // battery temperature time
    (0x414C, BatteryRole::History, false), // battery voltage statistics
    // What it is declared to be.
    (0x4058, BatteryRole::Configuration, false), // battery type
    (0xEF19, BatteryRole::Configuration, false), // battery type
    (0xEF14, BatteryRole::Configuration, false), // stop/start at charge mode
    (0xEF16, BatteryRole::Configuration, false), // monitor warning at engine off
    // The traction battery of a hybrid, on its own module.
    (0x4801, BatteryRole::Hybrid, true), // hybrid battery state of charge
    (0x4800, BatteryRole::Hybrid, true), // hybrid battery temperature
    (0x480D, BatteryRole::Hybrid, true), // hybrid battery pack voltage
    (0x4814, BatteryRole::Hybrid, false), // leakage resistance, contacts open
    (0x4815, BatteryRole::Hybrid, false), // charge power limits
    (0x4816, BatteryRole::Hybrid, false), // discharge power limits
    (0x4818, BatteryRole::Hybrid, false), // maintenance mode status
    (0x483E, BatteryRole::Hybrid, false), // variation in state of charge
    (0x483F, BatteryRole::Hybrid, false), // variation in voltage measurement
    (0x4840, BatteryRole::Hybrid, false), // minimum module voltage
    (0x4841, BatteryRole::Hybrid, false), // average module voltage
];

/// Words that make a name a battery or charging-system name. The check is
/// deliberately on the name SDD gives the identifier **on that module**, so
/// the same number meaning something else elsewhere is not admitted.
const BATTERY_WORDS: &[&str] = &[
    "batt",
    "charge",
    "charging",
    "discharg",
    "quies",
    "crank",
    "alternator",
    "generator",
    "amp hour",
    "amp_hr",
    "amp-hour",
    "contactor",
    "leakage",
    "relay box",
    "bms",
    "soc",
];

/// The role this identifier plays for the battery, or `None` when it is not
/// a battery parameter at all. Both tests must pass: the table holds the
/// identifier, and the name names the battery or the charging system.
pub fn battery_role(identifier: u16, parameter: &str) -> Option<BatteryRole> {
    let (_, role, _) = BATTERY_IDENTIFIERS
        .iter()
        .find(|(candidate, _, _)| *candidate == identifier)?;
    names_the_battery(parameter).then_some(*role)
}

/// Whether a reading belongs on the card's face rather than in its groups.
pub fn is_headline(identifier: u16) -> bool {
    BATTERY_IDENTIFIERS
        .iter()
        .any(|(candidate, _, headline)| *candidate == identifier && *headline)
}

/// Whether SDD's own name for a parameter names the battery or the charging
/// system.
pub fn names_the_battery(parameter: &str) -> bool {
    let name = parameter.to_ascii_lowercase();
    BATTERY_WORDS.iter().any(|word| name.contains(word))
}

/// SDD's own battery-voltage bands, from the battery monitor's configuration
/// (`BatteryMonitor.ini`), in millivolts. They are a fact about SDD and not
/// a measurement of any battery: this product shows them as SDD's, named as
/// SDD's, and derives no verdict of its own from them. The overlap between
/// the bands is SDD's own.
pub const SDD_VOLTAGE_LOW_MAX_MV: u32 = 11_600;
pub const SDD_VOLTAGE_MEDIUM_MIN_MV: u32 = 11_500;
pub const SDD_VOLTAGE_MEDIUM_MAX_MV: u32 = 13_000;
pub const SDD_VOLTAGE_HIGH_MIN_MV: u32 = 12_800;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_battery_monitor_dataset_is_admitted_by_identifier_and_by_name() {
        assert_eq!(
            battery_role(0x4028, "Vehicle Battery Estimated State of Charge"),
            Some(BatteryRole::Charge)
        );
        assert_eq!(
            battery_role(0x4029, "Vehicle Battery Temperature - Estimated"),
            Some(BatteryRole::Temperature)
        );
        assert_eq!(
            battery_role(
                0x4025,
                "Average Vehicle Quiesent Current - Previous 24 Hours (mA)"
            ),
            Some(BatteryRole::Drain)
        );
        assert_eq!(
            battery_role(
                0x41EA,
                "Estimated Cold Cranking Voltage At Present State Of Charge"
            ),
            Some(BatteryRole::Health)
        );
        assert_eq!(
            battery_role(0x4020, "Number of Vehicle Battery Monitor Resets"),
            Some(BatteryRole::History)
        );
        assert_eq!(
            battery_role(0x4801, "Hybrid Battery State of Charge"),
            Some(BatteryRole::Hybrid)
        );
        // SDD's own spelling of maintenance is its own; the identifier and
        // the word "battery" are what admit it.
        assert_eq!(
            battery_role(0x4818, "Battery Maintanence Mode Status"),
            Some(BatteryRole::Hybrid)
        );
    }

    #[test]
    fn a_name_that_only_contains_charge_is_not_the_battery() {
        // The one the word match would have let in.
        assert_eq!(
            battery_role(0xDE05, "Turbocharger valve offset values"),
            None
        );
        // In the table, but the module calls it something else entirely:
        // the number alone does not make it the battery.
        assert_eq!(battery_role(0x4028, "Steering angle sensor offset"), None);
        // Not in the table, however well it is named.
        assert_eq!(
            battery_role(0x1234, "Vehicle battery state of charge"),
            None
        );
    }

    #[test]
    fn the_face_of_the_card_is_the_few_readings_a_person_asks_for_first() {
        for identifier in [0x4028, 0x402A, 0x4090, 0x4029, 0x41EA, 0x4027, 0x4020] {
            assert!(
                is_headline(identifier),
                "0x{identifier:04X} belongs on the face"
            );
        }
        for identifier in [0x41F5, 0x409E, 0x4058, 0xEEB1] {
            assert!(
                !is_headline(identifier),
                "0x{identifier:04X} belongs in a group"
            );
        }
    }

    #[test]
    fn every_identifier_is_named_once_and_the_groups_are_ordered() {
        let mut seen = std::collections::BTreeSet::new();
        for (identifier, _, _) in BATTERY_IDENTIFIERS {
            assert!(
                seen.insert(*identifier),
                "0x{identifier:04X} is listed twice"
            );
        }
        assert_eq!(BatteryRole::Charge.order(), 0);
        assert!(BatteryRole::Hybrid.order() > BatteryRole::History.order());
    }

    /// SDD's bands are SDD's: recorded as they are, overlap and all, so that
    /// nothing here quietly becomes this product's own threshold.
    #[test]
    fn sdd_voltage_bands_are_recorded_as_sdd_wrote_them() {
        let bands = [
            SDD_VOLTAGE_LOW_MAX_MV,
            SDD_VOLTAGE_MEDIUM_MIN_MV,
            SDD_VOLTAGE_MEDIUM_MAX_MV,
            SDD_VOLTAGE_HIGH_MIN_MV,
        ];
        assert_eq!(bands, [11_600, 11_500, 13_000, 12_800]);
        // The bands overlap, and that overlap is SDD's own: a value of
        // 11,550 mV is both low and medium by its own configuration. Kept
        // as written rather than tidied into something SDD never said.
        assert!(bands[1] < bands[0], "the low and medium bands overlap");
        assert!(bands[3] < bands[2], "the medium and high bands overlap");
    }
}
