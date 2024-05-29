pub use rt_ss_bsp::{
    // RTIC uses `interrupt::{enable, disable}` to enable/disable interrupts globally
    riscv::interrupt,
    // RTIC requires the `Peripherals` definition with the `steal` method
    Peripherals,
};

use rt_ss_bsp::{
    clic::{InterruptNumber, Polarity, Trig, CLIC},
    interrupt::nested,
    riscv, sprintln, Interrupt,
};

#[cfg(all(feature = "riscv-atalanta", not(feature = "riscv-atalanta-backend")))]
compile_error!("Building for Atalanta, but 'riscv-atalanta-backend not selected'");

pub mod mintthresh {
    //! Read/write methods for RISC-V machine-mode interrupt threshold (`MINTTHRESH`)
    use rt_ss_bsp::riscv::{read_csr_as_usize, write_csr_as_usize};
    read_csr_as_usize!(0x347);
    write_csr_as_usize!(0x347);
}

/// Set the given software interrupt as pending
#[inline(always)]
pub fn pend(intr: Interrupt) {
    sprintln!("pend {:?} enter", intr);
    riscv::interrupt::free(|| unsafe { CLIC::ip(intr).pend() });
    sprintln!("pend {:?} leave", intr);
}

// Wrap the running task
///
/// On Atalanta/CLIC, `run` saves and restores the threshold register, unless we're at level floor
/// (1).
#[inline(always)]
pub fn run<F>(level: u8, f: F)
where
    F: FnOnce(),
{
    sprintln!("run task@{} enter", level);
    if level == 1 {
        // If level is 1, level threshold should be 1
        unsafe { nested(|| f()) };
        // Set interrupt threshold (`mintthresh` CSR) to 1
        mintthresh::write(1)
    } else {
        // Read current thresh
        let initial = mintthresh::read();
        mintthresh::write(level as usize);
        unsafe { nested(|| f()) };
        // Write back old thresh
        mintthresh::write(initial)
    }
    sprintln!("run task@{} leave", level);
}

/// Runs a function that takes a shared resource with a priority ceiling.
/// This function returns the return value of the target function.
///
/// # Safety
///
/// Input argument `ptr` must be a valid pointer to a shared resource.
#[inline]
pub unsafe fn lock<F, T, R>(ptr: *mut T, ceiling: u8, f: F) -> R
where
    F: FnOnce(&mut T) -> R,
{
    sprintln!("lock (ceiling={}) enter", ceiling);
    // We restore the previous threshold after the function is done
    let previous = mintthresh::read();
    mintthresh::write(ceiling as usize);
    let r = f(&mut *ptr);
    mintthresh::write(previous);
    sprintln!("lock (ceiling={}) leave", ceiling);
    r
}

pub fn enable(intr: Interrupt, level: u8) {
    //sprintln!("enable {:?} = {} @ level = {}", intr, intr.number(), level);
    CLIC::attr(intr).set_trig(Trig::Edge);
    CLIC::attr(intr).set_polarity(Polarity::Pos);
    CLIC::ctl(intr).set_level(level);
    CLIC::attr(intr).set_shv(true);
    unsafe { CLIC::ie(intr).enable() };
}
