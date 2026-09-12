//! The bench (ADR-0020): a vehicle standing in for a real one behind a
//! stand-in adapter. The vehicle sees CAN frames on a route and answers with
//! CAN frames — or, on a serial line, bytes in and bytes out (`ADR-0029`
//! slice B) — and it never sees the adapter's own protocol, as the adapter
//! never sees a vehicle. Whatever implements this is synthetic by
//! definition: it proves the software above it, never a car.

use crate::CanFrame;

/// The two CAN routes a bench vehicle can be reached on, named as the
/// application names them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BenchRoute {
    /// J1962 pins 6/14, 500 kbit/s.
    HsCan,
    /// J1962 pins 3/11, 125 kbit/s.
    MsCan,
    /// J1962 pin 7, the bus's own baud: the L322's DS2 body modules and the
    /// L316's ABS (`ADR-0029`).
    KLine7,
    /// J1962 pin 8: protocols this product does not speak, so nothing on the
    /// bench answers here either.
    KLine8,
}

impl BenchRoute {
    /// The route id the application uses (`hs-can`, `ms-can`).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::HsCan => "hs-can",
            Self::MsCan => "ms-can",
            Self::KLine7 => "k-line-7",
            Self::KLine8 => "k-line-8",
        }
    }

    /// Whether this route carries bytes rather than CAN frames.
    pub fn is_serial(self) -> bool {
        matches!(self, Self::KLine7 | Self::KLine8)
    }
}

/// A vehicle on the bench: frames in, frames out, per route.
pub trait BenchBus: Send {
    /// A frame the tester put on `route`; the frames the vehicle answers
    /// with, in order. An ISO-TP first frame is answered by its first frame
    /// only; the consecutive frames follow the tester's flow control, which
    /// arrives here as another frame.
    fn on_frame(&mut self, route: BenchRoute, frame: &CanFrame) -> Vec<CanFrame>;

    /// Frames the vehicle emits on its own while `route` is open — broadcast
    /// traffic. None by default: a bench that invents bus traffic would be
    /// inventing vehicle behaviour.
    fn tick(&mut self, route: BenchRoute) -> Vec<CanFrame> {
        let _ = route;
        Vec::new()
    }

    /// Bytes the tester put on a serial line; the bytes the vehicle answers
    /// with (`ADR-0029` slice B). A K-line echoes what the tester sends, so
    /// an answer begins with the request itself — that is the line's own
    /// behaviour and the protocol crates strip it. None by default: a bench
    /// without serial modules is silent on a serial line, as a car without
    /// them is.
    fn on_line_bytes(&mut self, route: BenchRoute, bytes: &[u8]) -> Vec<u8> {
        let _ = (route, bytes);
        Vec::new()
    }

    /// Whether any module on the bench answers on this serial line. The
    /// wake-up of a line with nobody on it times out, which is what a real
    /// adapter reports and what a tester should see.
    fn has_serial_modules(&self, route: BenchRoute) -> bool {
        let _ = route;
        false
    }

    /// One line about what is on the bench, for the adapter panel and the
    /// board-info evidence: the vehicle described, or that none is.
    fn describe(&self) -> String;
}

/// The bench with nothing on it: every frame goes unanswered.
#[derive(Debug, Default)]
pub struct EmptyBench;

impl BenchBus for EmptyBench {
    fn on_frame(&mut self, _route: BenchRoute, _frame: &CanFrame) -> Vec<CanFrame> {
        Vec::new()
    }

    fn describe(&self) -> String {
        "no vehicle on the bench".to_string()
    }
}
