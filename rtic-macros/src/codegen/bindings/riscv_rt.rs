use crate::{
    analyze::Analysis as CodegenAnalysis,
    syntax::{analyze::Analysis as SyntaxAnalysis, ast::App},
};
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{parse, Attribute, Ident};

pub fn interrupt_ident() -> Ident {
    let span = Span::call_site();
    Ident::new("Interrupt", span)
}

pub fn interrupt_mod(app: &App) -> TokenStream2 {
    let device = &app.args.device;
    let interrupt = interrupt_ident();
    quote!(#device::#interrupt)
}

pub fn impl_mutex(
    _app: &App,
    _analysis: &CodegenAnalysis,
    cfgs: &[Attribute],
    resources_prefix: bool,
    name: &Ident,
    ty: &TokenStream2,
    ceiling: u8,
    ptr: &TokenStream2,
) -> TokenStream2 {
    let path = if resources_prefix {
        quote!(shared_resources::#name)
    } else {
        quote!(#name)
    };

    quote!(
        #(#cfgs)*
        impl<'a> rtic::Mutex for #path<'a> {
            type T = #ty;

            #[inline(always)]
            fn lock<RTIC_INTERNAL_R>(&mut self, f: impl FnOnce(&mut #ty) -> RTIC_INTERNAL_R) -> RTIC_INTERNAL_R {

                const CEILING: u8 = #ceiling;

                unsafe {
                    rtic::export::lock(#ptr, CEILING, f)
                }
            }
        }
    )
}

pub fn extra_assertions(_app: &App, _analysis: &SyntaxAnalysis) -> Vec<TokenStream2> {
    vec![]
}

pub fn pre_init_preprocessing(_app: &mut App, _analysis: &SyntaxAnalysis) -> parse::Result<()> {
    Ok(())
}

pub fn pre_init_checks(app: &App, _analysis: &SyntaxAnalysis) -> Vec<TokenStream2> {
    let mut stmts = vec![];
    let int_mod = interrupt_mod(app);

    // Check that all dispatchers exists in the `#device::Interrupt` enumeration
    for name in app.args.dispatchers.keys() {
        stmts.push(quote!(let _ = #int_mod::#name;));
    }

    stmts
}

pub fn pre_init_enable_interrupts(app: &App, analysis: &CodegenAnalysis) -> Vec<TokenStream2> {
    let mut stmts = vec![];

    // First, we reset and disable all the interrupt controllers
    stmts.push(quote!(rtic::export::mintthresh::write(u8::MAX as usize);));

    // Then, we set the corresponding priorities
    let int_mod = interrupt_mod(app);
    let interrupt_ids = analysis.interrupts.iter().map(|(p, (id, _))| (p, id));
    for (&p, name) in interrupt_ids.chain(
        app.hardware_tasks
            .values()
            .map(|task| (&task.args.priority, &task.args.binds)),
    ) {
        stmts.push(quote!(
            rtic::export::enable(#int_mod::#name, #p);
        ));
    }

    // Finally, we activate the interrupts
    stmts.push(quote!(rtic::export::mintthresh::write(0x0);));

    stmts
}

pub fn architecture_specific_analysis(_app: &App, _analysis: &SyntaxAnalysis) -> parse::Result<()> {
    Ok(())
}

pub fn interrupt_entry(_app: &App, _analysis: &CodegenAnalysis) -> Vec<TokenStream2> {
    vec![]
}

pub fn interrupt_exit(_app: &App, _analysis: &CodegenAnalysis) -> Vec<TokenStream2> {
    vec![]
}

pub fn check_stack_overflow_before_init(
    _app: &App,
    _analysis: &CodegenAnalysis,
) -> Vec<TokenStream2> {
    vec![quote!(
        // Check for stack overflow using symbols from `riscv-rt`.
        extern "C" {
            pub static _stack_start: u32;
            pub static _ebss: u32;
        }
        let stack_start = &_stack_start as *const _ as u32;
        let ebss = &_ebss as *const _ as u32;
        if stack_start > ebss {
            // No flip-link usage, check the SP for overflow.
            if rtic::export::read_sp() <= ebss {
                panic!("pre-init sp ovrflw");
            }
        }
    )]
}

pub fn async_entry(
    _app: &App,
    _analysis: &CodegenAnalysis,
    _dispatcher_name: Ident,
) -> Vec<TokenStream2> {
    vec![]
}

pub fn async_prio_limit(_app: &App, analysis: &CodegenAnalysis) -> Vec<TokenStream2> {
    let max = if let Some(max) = analysis.max_async_prio {
        quote!(#max)
    } else {
        // No limit
        quote!(u8::MAX)
    };

    vec![quote!(
        /// Holds the maximum priority level for use by async HAL drivers.
        #[no_mangle]
        static RTIC_ASYNC_MAX_LOGICAL_PRIO: u8 = #max;
    )]
}

pub fn handler_config(
    _app: &App,
    _analysis: &CodegenAnalysis,
    _dispatcher_name: Ident,
) -> Vec<TokenStream2> {
    let mut stmts = vec![];

    stmts.push(quote!(#[bsp::rt::interrupt]));

    stmts
}

pub fn extra_modules(_app: &App, _analysis: &SyntaxAnalysis) -> Vec<TokenStream2> {
    vec![]
}
