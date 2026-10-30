use std::{
    fs::File,
    process::{Command, Stdio, exit},
    sync::atomic::{AtomicBool, Ordering::SeqCst},
};

use crate::ui;

static CRITICAL: AtomicBool = AtomicBool::new(false);
static PENDING: AtomicBool = AtomicBool::new(false);
static EXITING: AtomicBool = AtomicBool::new(false);
static CHILD: AtomicBool = AtomicBool::new(false);

pub(crate) fn install() {
    let _ = ctrlc::set_handler(|| {
        if CHILD.load(SeqCst) {
            return;
        }
        if CRITICAL.load(SeqCst) {
            PENDING.store(true, SeqCst);
        } else {
            interrupted();
        }
    });
}

pub(crate) fn interrupted() -> ! {
    // the handler thread and a prompt can both see the same ctrl-c
    if EXITING.swap(true, SeqCst) {
        loop {
            std::thread::park();
        }
    }
    // ctrl-c during a password prompt would otherwise leave the terminal without echo
    if let Ok(tty) = File::open("/dev/tty") {
        let _ = Command::new("stty")
            .arg("echo")
            .stdin(tty)
            .stderr(Stdio::null())
            .status();
    }
    eprintln!("\n{}", ui::edim("interrupted"));
    exit(130);
}

/// defers ctrl-c until dropped, so a slate is never left half-written
pub(crate) struct Critical;

impl Critical {
    pub(crate) fn enter() -> Self {
        CRITICAL.store(true, SeqCst);
        Critical
    }
}

/// leaves ctrl-c to a foreground child (ssh) until dropped, so cleanup still runs
pub(crate) struct Child;

impl Child {
    pub(crate) fn enter() -> Self {
        CHILD.store(true, SeqCst);
        Child
    }
}

impl Drop for Child {
    fn drop(&mut self) {
        CHILD.store(false, SeqCst);
    }
}

impl Drop for Critical {
    fn drop(&mut self) {
        CRITICAL.store(false, SeqCst);
        if PENDING.load(SeqCst) {
            interrupted();
        }
    }
}
