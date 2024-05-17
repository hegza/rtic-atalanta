use crate::{
    analyze::Analysis as CodegenAnalysis,
    codegen::util,
    syntax::{analyze::Analysis as SyntaxAnalysis, ast::App},
};
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{parse, Attribute, Ident};

#[allow(clippy::too_many_arguments)]
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
                /// Priority ceiling
                const CEILING: u8 = #ceiling;
                unsafe {
                    rtic::export::lock(
                        #ptr,
                        CEILING,
                        f,
                    )
                }
            }
        }
    )
}

pub fn interrupt_ident() -> Ident {
    let span = Span::call_site();
    Ident::new("Interrupt", span)
}

pub fn interrupt_mod(app: &App) -> TokenStream2 {
    let device = &app.args.device;
    let interrupt = interrupt_ident();
    quote!(#device::#interrupt)
}

pub fn extra_assertions(_: &App, _: &SyntaxAnalysis) -> Vec<TokenStream2> {
    vec![]
}

pub fn pre_init_preprocessing(_app: &mut App, _analysis: &SyntaxAnalysis) -> parse::Result<()> {
    Ok(())
}

pub fn pre_init_checks(app: &App, _: &SyntaxAnalysis) -> Vec<TokenStream2> {
    let mut stmts = vec![];
    // check that all dispatchers exists in the `Interrupt` enumeration regardless of whether
    // they are used or not
    let rt_err = util::rt_err_ident();

    for name in app.args.dispatchers.keys() {
        stmts.push(quote!(let _ = #rt_err::Interrupt::#name;));
    }
    stmts
}
pub fn pre_init_enable_interrupts(app: &App, analysis: &CodegenAnalysis) -> Vec<TokenStream2> {
    let mut stmts = vec![];

    // TODO: First, we reset and disable all the interrupt controllers
    stmts.push(quote! {
        rtic::export::clear_interrupts();
        rtic::export::interrupt::disable();
    });

    // Then, we set the corresponding priorities
    let interrupt_ids = analysis.interrupts.iter().map(|(p, (id, _))| (p, id));
    for (&p, name) in interrupt_ids.chain(
        app.hardware_tasks
            .values()
            .map(|task| (&task.args.priority, &task.args.binds)),
    ) {
        stmts.push(quote!(
            rtic::export::enable(bsp::interrupt::Interrupt::#name, #p);
        ));
    }
    // Finally, we activate the interrupts
    stmts.push(quote! {
        rtic::export::set_interrupts();
        rtic::export::interrupt::enable();
    });
    stmts
}

/// Any additional checks that depend on the system architecture.
pub fn architecture_specific_analysis(app: &App, _analysis: &SyntaxAnalysis) -> parse::Result<()> {
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
        // Check for stack overflow using symbols from `risc-v-rt`.
        extern "C" {
            pub static _stack_start: u32;
            pub static _bss_end: u32;
        }

        let stack_start = &_stack_start as *const _ as u32;
        let ebss = &_bss_end as *const _ as u32;

        if stack_start > ebss {
            // No flip-link usage, check the SP for overflow.
            if rtic::export::read_sp() <= ebss {
                panic!("Stack overflow after allocating executors");
            }
        }
    )]
}

pub fn async_entry(
    _app: &App,
    _analysis: &CodegenAnalysis,
    dispatcher_name: Ident,
) -> Vec<TokenStream2> {
    let mut stmts = vec![];
    stmts.push(quote!(
        rtic::export::unpend(rtic::export::Interrupt::#dispatcher_name); //simulate cortex-m behavior by unpending the interrupt on entry.
    ));
    stmts
}

pub fn async_prio_limit(app: &App, analysis: &CodegenAnalysis) -> Vec<TokenStream2> {
    let max = if let Some(max) = analysis.max_async_prio {
        quote!(#max)
    } else {
        // No limit
        let device = &app.args.device;
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
    vec![]
}

pub fn extra_modules(_app: &App, _analysis: &SyntaxAnalysis) -> Vec<TokenStream2> {
    vec![]
}
