use log::info;
use sfsm::*;
pub struct Startup {}
pub struct Initialising {}
pub struct Idle {}

add_state_machine!(
    pub MotionSensorState,                          // Name of the state machine. Accepts a visibility modifier.
    Startup,                   // The initial state the state machine will start in
    [Startup, Initialising,Idle],         // All possible states
    [
        Startup => Initialising,     // All transitions
        Initialising => Idle,     // All transitions
    ]
);

impl State for Startup {}
impl State for Initialising {}
impl State for Idle {}

impl Into<Initialising> for Startup {
    fn into(self) -> Initialising {
        Initialising {}
    }
}
impl Transition<Initialising> for Startup {
    // fn action(&mut self) {
    //     info!("Startup => Initialising");
    // }
    fn guard(&self) -> TransitGuard {
        info!("Startup => Initialising: Guard");
        return TransitGuard::Transit;
    }
}

impl Into<Idle> for Initialising {
    fn into(self) -> Idle {
        Idle {}
    }
}
impl Transition<Idle> for Initialising {
    // fn action(&mut self) {
    //     info!("Startup => Initialising");
    // }
    fn guard(&self) -> TransitGuard {
        info!("Initialising => Idle: Guard");
        return TransitGuard::Transit;
    }
}
