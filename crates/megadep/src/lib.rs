#![feature(rustc_private)]

extern crate rustc_ast;

// Load rustc as a plugin to get macros
extern crate rustc_driver;
#[macro_use]
extern crate rustc_lint;
#[macro_use]
extern crate rustc_session;

use rustc_ast::ast;
use rustc_driver::plugin::Registry;
use rustc_lint::{LateContext, LateLintPass, LintArray, LintContext, LintPass};
declare_lint!(MEGADEP, Warn, "A hack for great good");

declare_lint_pass!(Pass => [MEGADEP]);

impl LateLintPass<'tcx> for Pass {
    fn check_crate(&mut self, tcx: &LateContext<'tcx>) {
        dbg!("MEGADEP!");
    }
}

#[no_mangle]
fn __rustc_plugin_registrar(reg: &mut Registry) {
    reg.lint_store.register_lints(&[&MEGADEP]);
    reg.lint_store.register_late_pass(|| Box::new(Pass));
}
