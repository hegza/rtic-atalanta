pub use rt_ss_bsp::{
    // RTIC uses `interrupt::{enable, disable}` to enable/disable interrupts globally
    riscv::interrupt,
    // RTIC requires the `Peripherals` definition with the `steal` method
    Peripherals,
};

use rt_ss_bsp::{
    clic::{Polarity, Trig, CLIC},
    riscv, Interrupt,
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
    riscv::interrupt::free(|| unsafe { CLIC::ip(intr).pend() });
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
    if level == 1 {
        // If level is 1, level threshold should be 1
        f();
        // Set interrupt threshold (`mintthresh` CSR) to 1
        mintthresh::write(1)
    } else {
        // Read current thresh
        let initial = mintthresh::read();
        f();
        // Write back old thresh
        mintthresh::write(initial)
    }
}

pub fn enable(intr: Interrupt, level: u8) {
    CLIC::attr(intr).set_trig(Trig::Edge);
    CLIC::attr(intr).set_polarity(Polarity::Pos);
    CLIC::ctl(intr).set_level(level);
    CLIC::attr(intr).set_shv(true);
    unsafe { CLIC::ie(intr).enable() };
}
