//! The bench (ADR-0020): a vehicle standing in for a real one behind a
//! stand-in adapter. The vehicle sees CAN frames on a route and answers with
//! CAN frames; it never sees the adapter's own protocol, and the adapter never
//! sees a vehicle. Whatever implements this is synthetic by definition: it
//! proves the software above it, never a car.

use crate::CanFrame;

/// The two CAN routes a bench vehicle can be reached on, named as the
/// application names them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BenchRoute {
    /// J1962 pins 6/14, 500 kbit/s.
    HsCan,
    /// J1962 pins 3/11, 125 kbit/s.
    MsCan,
}

impl BenchRoute {
    /// The route id the application uses (`hs-can`, `ms-can`).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::HsCan => "hs-can",
            Self::MsCan => "ms-can",
        }
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
