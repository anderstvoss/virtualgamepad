//! Bounded typed callback and reply primitives for controller packages.
#![allow(clippy::missing_errors_doc, clippy::needless_pass_by_value)]

use std::{
    marker::PhantomData,
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{Receiver, SyncSender, TrySendError, sync_channel},
    },
    thread::{self, JoinHandle},
};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallbackDiagnostics {
    pub delivered: u64,
    pub dropped: u64,
    pub panics: u64,
    pub closed: bool,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SubscriptionError {
    #[error("subscription is closed")]
    Closed,
    #[error("subscription queue is full")]
    Full,
}

struct SubscriptionState {
    delivered: AtomicU64,
    dropped: AtomicU64,
    panics: AtomicU64,
    closed: AtomicBool,
}

/// A bounded, isolated typed callback worker.
pub struct ReverseSubscription<E: Send + 'static> {
    sender: Mutex<Option<SyncSender<E>>>,
    state: Arc<SubscriptionState>,
    worker: Mutex<Option<JoinHandle<()>>>,
    worker_id: thread::ThreadId,
}
impl<E: Send + 'static> ReverseSubscription<E> {
    #[must_use]
    pub fn new(capacity: usize, mut callback: impl FnMut(E) + Send + 'static) -> Self {
        let (sender, receiver) = sync_channel(capacity);
        let state = Arc::new(SubscriptionState {
            delivered: AtomicU64::new(0),
            dropped: AtomicU64::new(0),
            panics: AtomicU64::new(0),
            closed: AtomicBool::new(false),
        });
        let worker_state = Arc::clone(&state);
        let worker =
            thread::spawn(move || run_subscription(receiver, &worker_state, &mut callback));
        Self {
            sender: Mutex::new(Some(sender)),
            state,
            worker_id: worker.thread().id(),
            worker: Mutex::new(Some(worker)),
        }
    }
    pub fn publish(&self, event: E) -> Result<(), SubscriptionError> {
        if self.state.closed.load(Ordering::Acquire) {
            return Err(SubscriptionError::Closed);
        }
        let sender = self
            .sender
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let Some(sender) = sender.as_ref() else {
            return Err(SubscriptionError::Closed);
        };
        match sender.try_send(event) {
            Ok(()) => Ok(()),
            Err(TrySendError::Full(_)) => {
                self.state.dropped.fetch_add(1, Ordering::Relaxed);
                Err(SubscriptionError::Full)
            }
            Err(TrySendError::Disconnected(_)) => Err(SubscriptionError::Closed),
        }
    }
    /// Stop publication and drain accepted events before returning. A callback
    /// may close its own subscription; that call disconnects without self-join.
    pub fn close(&self) {
        self.state.closed.store(true, Ordering::Release);
        self.sender
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
        if thread::current().id() == self.worker_id {
            return;
        }
        // Serialize external closers through the join, so each waits for cleanup.
        let mut worker_slot = self
            .worker
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(worker) = worker_slot.take() {
            let _ = worker.join();
        }
    }
    #[must_use]
    pub fn diagnostics(&self) -> CallbackDiagnostics {
        CallbackDiagnostics {
            delivered: self.state.delivered.load(Ordering::Relaxed),
            dropped: self.state.dropped.load(Ordering::Relaxed),
            panics: self.state.panics.load(Ordering::Relaxed),
            closed: self.state.closed.load(Ordering::Acquire),
        }
    }
}
impl<E: Send + 'static> Drop for ReverseSubscription<E> {
    fn drop(&mut self) {
        self.close();
    }
}
fn run_subscription<E>(
    receiver: Receiver<E>,
    state: &SubscriptionState,
    callback: &mut impl FnMut(E),
) {
    while let Ok(event) = receiver.recv() {
        if catch_unwind(AssertUnwindSafe(|| callback(event))).is_err() {
            state.panics.fetch_add(1, Ordering::Relaxed);
            break;
        }
        state.delivered.fetch_add(1, Ordering::Relaxed);
    }
    state.closed.store(true, Ordering::Release);
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ReplyError {
    #[error("reply token was already used")]
    AlreadyReplied,
    #[error("reply channel is closed")]
    Closed,
    #[error("reply queue is full")]
    Full,
}

/// A typed one-shot response token. The type parameter prevents controller
/// families from replying with an incompatible payload.
pub struct ReplyToken<R> {
    id: u64,
    sender: SyncSender<(u64, R)>,
    used: Arc<AtomicBool>,
    _type: PhantomData<fn(R)>,
}
impl<R> ReplyToken<R> {
    #[must_use]
    pub const fn id(&self) -> u64 {
        self.id
    }
    pub fn reply(self, reply: R) -> Result<(), ReplyError> {
        if self.used.swap(true, Ordering::AcqRel) {
            return Err(ReplyError::AlreadyReplied);
        }
        match self.sender.try_send((self.id, reply)) {
            Ok(()) => Ok(()),
            Err(TrySendError::Full(_)) => Err(ReplyError::Full),
            Err(TrySendError::Disconnected(_)) => Err(ReplyError::Closed),
        }
    }
}
pub struct ReplyInbox<R> {
    sender: SyncSender<(u64, R)>,
    receiver: Receiver<(u64, R)>,
    next: AtomicU64,
}
impl<R> ReplyInbox<R> {
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        let (sender, receiver) = sync_channel(capacity);
        Self {
            sender,
            receiver,
            next: AtomicU64::new(1),
        }
    }
    #[must_use]
    pub fn issue(&self) -> ReplyToken<R> {
        ReplyToken {
            id: self.next.fetch_add(1, Ordering::Relaxed),
            sender: self.sender.clone(),
            used: Arc::new(AtomicBool::new(false)),
            _type: PhantomData,
        }
    }
    pub fn try_recv(&self) -> Option<(u64, R)> {
        self.receiver.try_recv().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        sync::atomic::{AtomicUsize, Ordering},
        time::Duration,
    };

    #[test]
    fn bounded_callback_delivery_contains_panics() {
        let calls = Arc::new(AtomicUsize::new(0));
        let observed = Arc::clone(&calls);
        let subscription = ReverseSubscription::new(2, move |value: u8| {
            observed.fetch_add(usize::from(value), Ordering::Relaxed);
            assert!(value != 2, "injected");
        });
        subscription.publish(1).expect("first event");
        subscription.publish(2).expect("second event");
        std::thread::sleep(Duration::from_millis(20));
        assert!(subscription.diagnostics().panics <= 1);
        subscription.close();
        assert!(subscription.diagnostics().closed);
    }
    #[test]
    fn full_queue_close_and_drop_drain_and_terminate() {
        for drop_only in [false, true] {
            let (started_tx, started) = sync_channel(1);
            let (release, release_rx) = sync_channel(1);
            let calls = Arc::new(AtomicUsize::new(0));
            let observed = Arc::clone(&calls);
            let subscription = ReverseSubscription::new(1, move |value| {
                if value == 1 {
                    started_tx.send(()).unwrap();
                    release_rx.recv().unwrap();
                }
                observed.fetch_add(1, Ordering::Relaxed);
            });
            subscription.publish(1).unwrap();
            started.recv_timeout(Duration::from_secs(2)).unwrap();
            subscription.publish(2).unwrap();
            let (done_tx, done) = sync_channel(1);
            let closer = thread::spawn(move || {
                if !drop_only {
                    subscription.close();
                    subscription.close();
                }
                drop(subscription);
                done_tx.send(()).unwrap();
            });
            release.send(()).unwrap();
            done.recv_timeout(Duration::from_secs(2)).unwrap();
            closer.join().unwrap();
            assert_eq!(calls.load(Ordering::Relaxed), 2);
        }
    }

    #[test]
    fn zero_capacity_and_concurrent_close_terminate() {
        let subscription = Arc::new(ReverseSubscription::new(0, |_: u8| {}));
        let other = Arc::clone(&subscription);
        let (done_tx, done) = sync_channel(1);
        let closer = thread::spawn(move || {
            other.close();
            done_tx.send(()).unwrap();
        });
        subscription.close();
        done.recv_timeout(Duration::from_secs(2)).unwrap();
        closer.join().unwrap();
        assert_eq!(subscription.publish(1), Err(SubscriptionError::Closed));
    }

    #[test]
    fn publishing_racing_close_drains_every_accepted_event() {
        let accepted = Arc::new(AtomicUsize::new(0));
        let delivered = Arc::new(AtomicUsize::new(0));
        let observed = Arc::clone(&delivered);
        let subscription = Arc::new(ReverseSubscription::new(256, move |_: u8| {
            observed.fetch_add(1, Ordering::Relaxed);
        }));
        let barrier = Arc::new(std::sync::Barrier::new(3));
        let publishers: Vec<_> = (0..2)
            .map(|_| {
                let subscription = Arc::clone(&subscription);
                let accepted = Arc::clone(&accepted);
                let barrier = Arc::clone(&barrier);
                thread::spawn(move || {
                    barrier.wait();
                    for _ in 0..128 {
                        match subscription.publish(1) {
                            Ok(()) => {
                                accepted.fetch_add(1, Ordering::Relaxed);
                            }
                            Err(SubscriptionError::Closed) => break,
                            Err(SubscriptionError::Full) => {
                                panic!("queue holds all submitted events")
                            }
                        }
                    }
                })
            })
            .collect();
        barrier.wait();
        subscription.close();
        for publisher in publishers {
            publisher.join().unwrap();
        }
        assert_eq!(
            accepted.load(Ordering::Relaxed),
            delivered.load(Ordering::Relaxed)
        );
        assert_eq!(subscription.publish(1), Err(SubscriptionError::Closed));
    }

    #[test]
    fn callback_can_close_its_own_subscription() {
        let (reference_tx, reference) = sync_channel::<std::sync::Weak<ReverseSubscription<u8>>>(1);
        let (done_tx, done) = sync_channel(1);
        let subscription = Arc::new(ReverseSubscription::new(1, move |_: u8| {
            reference.recv().unwrap().upgrade().unwrap().close();
            done_tx.send(()).unwrap();
        }));
        reference_tx.send(Arc::downgrade(&subscription)).unwrap();
        subscription.publish(1).unwrap();
        done.recv_timeout(Duration::from_secs(2)).unwrap();
        subscription.close();
        assert_eq!(subscription.diagnostics().delivered, 1);
    }

    #[test]
    fn typed_reply_token_is_one_shot() {
        let inbox = ReplyInbox::<u16>::new(1);
        let token = inbox.issue();
        let id = token.id();
        token.reply(7).expect("first reply");
        assert_eq!(inbox.try_recv(), Some((id, 7)));
    }
}
