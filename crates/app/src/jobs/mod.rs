//! Scheduled jobs. The binary calls [`Scheduler::tick`] every minute; each
//! job runs once per local date after its time, claimed in `job_runs` so a
//! restart or a second process never sends a report twice.

mod kind;
mod runner;
mod scheduler;

pub use kind::JobKind;
pub use runner::{JobError, JobRunner, MAX_BACKUP_SILENCE_HOURS};
pub use scheduler::{MAX_JOB_ATTEMPTS, Scheduler};

#[cfg(test)]
mod tests;
