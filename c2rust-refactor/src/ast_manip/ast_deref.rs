use rustc_target::spec::abi::Abi;
use syntax::ast::*;
use syntax::token::{BinOpToken, DelimToken, Nonterminal, Token, TokenKind};
use syntax::token::{Lit as TokenLit, LitKind as TokenLitKind};
use syntax::ptr::P;
use syntax::source_map::{Span, Spanned};
use syntax::tokenstream::{DelimSpan, TokenStream, TokenTree};
use syntax::ThinVec;
use syntax_pos::hygiene::SyntaxContext;

pub trait AstDeref {
    type Target: ?Sized;
    fn ast_deref(&self) -> &Self::Target;
}

include!(concat!(env!("OUT_DIR"), "/ast_deref_gen.inc.rs"));

impl<T: AstDeref> AstDeref for P<T> {
    type Target = <T as AstDeref>::Target;
    fn ast_deref(&self) -> &Self::Target {
        <T as AstDeref>::ast_deref(self)
    }
}

impl<T: AstDeref> AstDeref for Spanned<T> {
    type Target = <T as AstDeref>::Target;
    fn ast_deref(&self) -> &Self::Target {
        self.node.ast_deref()
    }
}

impl<T> AstDeref for Vec<T> {
    type Target = [T];
    fn ast_deref(&self) -> &Self::Target {
        &**self
    }
}

impl<T> AstDeref for ThinVec<T> {
    type Target = [T];
    fn ast_deref(&self) -> &Self::Target {
        &**self
    }
}
