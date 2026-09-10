use gr_hid::{Command, Delivery, Error, HostEvent, Transport};
use std::{cell::RefCell, collections::VecDeque, rc::Rc};

#[derive(Default)]
pub struct Io {
    pub events: VecDeque<HostEvent>,
    pub outcomes: VecDeque<Delivery>,
    pub attempts: Vec<Command>,
    pub submitted: Vec<Command>,
    pub closes: usize,
    pub close_fails: bool,
}
#[derive(Clone, Default)]
pub struct Fake(pub Rc<RefCell<Io>>);
impl Transport for Fake {
    fn event(&mut self) -> Result<Option<HostEvent>, Error> {
        Ok(self.0.borrow_mut().events.pop_front())
    }
    fn submit(&mut self, command: &Command) -> Delivery {
        let mut io = self.0.borrow_mut();
        io.attempts.push(command.clone());
        let outcome = io.outcomes.pop_front().unwrap_or(Delivery::Submitted);
        if outcome == Delivery::Submitted {
            io.submitted.push(command.clone());
        }
        outcome
    }
    fn close(&mut self) -> Result<(), Error> {
        let mut io = self.0.borrow_mut();
        io.closes += 1;
        if io.close_fails {
            Err(Error::Transport)
        } else {
            Ok(())
        }
    }
}
