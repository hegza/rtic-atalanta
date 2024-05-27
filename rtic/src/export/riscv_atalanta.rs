pub use rt_ss_bsp::{interrupt::Interrupt, Peripherals};

use riscv_peripheral::clic::intattr::{Polarity, Trig};
use rt_ss_bsp::{clic::CLIC, riscv, sprintln, uart::uart_write};

#[cfg(all(feature = "riscv-atalanta", not(feature = "riscv-atalanta-backend")))]
compile_error!("Building for Atalanta, but 'riscv-atalanta-backend not selected'");

mod mintthresh {
    use rt_ss_bsp::riscv::{read_csr_as_usize, write_csr_as_usize};
    read_csr_as_usize!(0x347);
    write_csr_as_usize!(0x347);
}

pub fn global_enable() {
    uart_write("mstatus:en\r\n");
    unsafe { riscv::interrupt::enable() }
}

pub fn global_disable() {
    uart_write("mstatus:disable\r\n");
    riscv::interrupt::disable()
}

/// Wrap the running task
///
/// On Atalanta/CLIC, `run` saves and restores the threshold register, unless we're at level floor
/// (1).
#[inline(always)]
pub fn run<F>(level: u8, f: F)
where
    F: FnOnce(),
{
    //uart_write("run enter\r\n");
    if level == 1 {
        // If level is 1, level thresh should be 1
        f();
        // Set interrupt threshold (`mintthresh` CSR) to 1
        mintthresh::write(0x1)
    } else {
        // Read current thresh
        let initial = mintthresh::read();
        f();
        // Write back old thresh
        mintthresh::write(initial)
    }
}

/// Lock implementation using threshold and global Critical Section (CS)
///
/// # Safety
///
/// The system ceiling is raised from current to ceiling
/// by either
/// - raising the threshold to the ceiling value, or
/// - disable all interrupts in case we want to
///   mask interrupts with maximum level
///
/// Dereferencing a raw pointer inside CS
///
/// The level.set/level.get can safely be outside the CS
/// as being a context local cell (not affected by preemptions).
/// It is merely used in order to omit masking in case current
/// level is current level >= ceiling.
#[inline(always)]
pub unsafe fn lock<T, R>(ptr: *mut T, ceiling: u8, f: impl FnOnce(&mut T) -> R) -> R {
    if ceiling == (15) {
        // turn off interrupts completely, were at max prio
        let r = critical_section::with(|_| f(&mut *ptr));
        r
    } else {
        let current = mintthresh::read();

        // Atalanta/RT-Ibex lets interrupts with prio equal to threshold through so we up it by one
        mintthresh::write((ceiling + 1) as usize);
        let r = f(&mut *ptr);
        mintthresh::write(current as usize);
        r
    }
}

/// Set the given software interrupt as pending
#[inline(always)]
pub fn pend(intr: Interrupt) {
    uart_write("pend\r\n");
    // Wrapping the pend call with mintthresh raise & lower or a global intr disable seems to be
    // mandatory for proper operation
    riscv::interrupt::free(|| unsafe { CLIC::ip(intr).pend() });
}

/// Set the given software interrupt as not pending
pub fn unpend(intr: Interrupt) {
    unsafe { CLIC::ip(intr).unpend() }
}

pub fn set_level(intr: Interrupt, level: u8) {
    CLIC::ctl(intr).set_level(level);
}

pub fn enable(intr: Interrupt, level: u8) {
    //sprintln!("en intr ? at level {}", level);
    //sprintln!("en {:?}", intr);
    CLIC::attr(intr).set_trig(Trig::Edge);
    CLIC::attr(intr).set_polarity(Polarity::Pos);
    CLIC::ctl(intr).set_level(level);
    CLIC::attr(intr).set_shv(true);
    unsafe { CLIC::ie(intr).enable() };
}

pub fn disable(intr: Interrupt) {
    CLIC::ie(intr).disable();
    CLIC::ctl(intr).set_level(0x0);
    CLIC::attr(intr).set_shv(false);
    CLIC::attr(intr).set_trig(Trig::Level);
    CLIC::attr(intr).set_polarity(Polarity::Pos);
}

pub fn set_interrupts() {
    CLIC::smclicconfig().set_mnlbits(8);
    uart_write("mintthresh <- 0x0\r\n");
    mintthresh::write(0x0);
}

pub fn clear_interrupts() {
    uart_write("mintthresh <- 0xff\r\n");
    mintthresh::write(0xff);
}
