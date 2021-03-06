use rlua::prelude::{LuaResult, LuaString, LuaTable, LuaValue};
use rustc_target::spec::abi::Abi;
use syntax::ast::{
    Arg, BindingMode, Block, Crate, Expr, ExprKind, FloatTy, FnDecl, ImplItem, ImplItemKind,
    Item, ItemKind, LitKind, Local, Mod, Mutability::*, NodeId, Pat, PatKind, Path, PathSegment,
    Stmt, StmtKind, UintTy, IntTy, LitIntType, Ident, DUMMY_NODE_ID, BinOpKind, UnOp, BlockCheckMode,
    Label, StrStyle, TyKind, Ty, MutTy, Unsafety, FunctionRetTy, BareFnTy, UnsafeSource::*, Field,
    AnonConst, Lifetime, AngleBracketedArgs, GenericArgs, GenericArg,
};
use syntax::source_map::symbol::Symbol;
use syntax::source_map::{DUMMY_SP, Spanned};
use syntax::ptr::P;
use syntax::ThinVec;

use std::rc::Rc;

use crate::ast_manip::fn_edit::{FnKind, FnLike};

fn dummy_spanned<T>(node: T) -> Spanned<T> {
    Spanned {
        node,
        span: DUMMY_SP,
    }
}

fn dummy_expr() -> P<Expr> {
    P(Expr {
        attrs: ThinVec::new(),
        id: DUMMY_NODE_ID,
        node: ExprKind::Err,
        span: DUMMY_SP,
    })
}

fn dummy_block() -> P<Block> {
    P(Block {
        id: DUMMY_NODE_ID,
        rules: BlockCheckMode::Default,
        span: DUMMY_SP,
        stmts: Vec::new(),
    })
}

fn dummy_ty() -> P<Ty> {
    P(Ty {
        id: DUMMY_NODE_ID,
        node: TyKind::Err,
        span: DUMMY_SP,
    })
}

fn dummy_fn_decl() -> P<FnDecl> {
    P(FnDecl {
        inputs: Vec::new(),
        output: FunctionRetTy::Default(DUMMY_SP),
        c_variadic: false,
    })
}

fn dummy_pat() -> P<Pat> {
    P(Pat {
        id: DUMMY_NODE_ID,
        node: PatKind::Wild,
        span: DUMMY_SP,
    })
}

fn dummy_path() -> Path {
    Path {
        span: DUMMY_SP,
        segments: Vec::new(),
    }
}

fn dummy_generic_args() -> GenericArgs {
    GenericArgs::AngleBracketed(AngleBracketedArgs {
        span: DUMMY_SP,
        args: Vec::new(),
        bindings: Vec::new(),
    })
}

fn dummy_generic_arg() -> GenericArg {
    GenericArg::Lifetime(Lifetime {
        id: DUMMY_NODE_ID,
        ident: Ident::from_str("placeholder"),
    })
}

pub(crate) trait MergeLuaAst {
    fn merge_lua_ast<'lua>(&mut self, table: LuaTable<'lua>) -> LuaResult<()>;
}

impl MergeLuaAst for FnLike {
    fn merge_lua_ast<'lua>(&mut self, table: LuaTable<'lua>) -> LuaResult<()> {
        self.kind = match table.get::<_, String>("kind")?.as_str() {
            "Normal" => FnKind::Normal,
            "ImplMethod" => FnKind::ImplMethod,
            "TraitMethod" => FnKind::TraitMethod,
            "Foreign" => FnKind::Foreign,
            _ => self.kind,
        };
        self.id = NodeId::from_u32(table.get("id")?);
        self.ident.name = Symbol::intern(&table.get::<_, String>("ident")?);
        self.decl.merge_lua_ast(table.get("decl")?)?;

        // REVIEW: How do we deal with spans if there is no existing block
        // to modify?
        if let Some(ref mut block) = self.block {
            block.merge_lua_ast(table.get("block")?)?;
        }

        // TODO: Attrs

        Ok(())
    }
}

impl MergeLuaAst for P<Block> {
    fn merge_lua_ast<'lua>(&mut self, table: LuaTable<'lua>) -> LuaResult<()> {
        let lua_stmts: LuaTable = table.get("stmts")?;

        self.rules = match table.get::<_, String>("rules")?.as_str() {
            "Default" => BlockCheckMode::Default,
            "CompilerGeneratedUnsafe" => BlockCheckMode::Unsafe(CompilerGenerated),
            "UserProvidedUnsafe" => BlockCheckMode::Unsafe(UserProvided),
            e => unimplemented!("Found unknown block rule: {}", e),
        };

        // TODO: This may need to be improved if we want to delete or add
        // stmts as it currently expects stmts to be 1-1
        self.stmts.iter_mut().enumerate().map(|(i, stmt)| {
            let stmt_table: LuaTable = lua_stmts.get(i + 1)?;

            stmt.merge_lua_ast(stmt_table)
        }).collect()
    }
}

impl MergeLuaAst for Stmt {
    fn merge_lua_ast<'lua>(&mut self, table: LuaTable<'lua>) -> LuaResult<()> {
        // REVIEW: How do we deal with modifying to a different type of stmt than
        // the existing one?

        match self.node {
            StmtKind::Local(ref mut l) => l.merge_lua_ast(table)?,
            StmtKind::Item(ref mut i) => i.merge_lua_ast(table.get("item")?)?,
            StmtKind::Expr(ref mut e) |
            StmtKind::Semi(ref mut e) => e.merge_lua_ast(table.get("expr")?)?,
            _ => warn!("MergeLuaAst unimplemented for Macro StmtKind"),
        };

        Ok(())
    }
}

impl MergeLuaAst for P<FnDecl> {
    fn merge_lua_ast<'lua>(&mut self, table: LuaTable<'lua>) -> LuaResult<()> {
        let lua_args: LuaTable = table.get("args")?;
        let lua_return_type: Option<LuaTable> = table.get("return_type")?;
        let return_type = lua_return_type.map(|lua_ty| {
            let mut ty = dummy_ty();

            ty.merge_lua_ast(lua_ty).map(|_| ty)
        }).transpose()?;

        self.c_variadic = table.get("c_variadic")?;
        self.output = match return_type {
            Some(ty) => FunctionRetTy::Ty(ty),
            None => FunctionRetTy::Default(DUMMY_SP),
        };

        let mut args = Vec::new();

        for lua_arg in lua_args.sequence_values::<LuaTable>() {
            let mut arg = Arg {
                ty: dummy_ty(),
                pat: dummy_pat(),
                id: DUMMY_NODE_ID,
            };

            arg.merge_lua_ast(lua_arg?)?;
            args.push(arg);
        }

        self.inputs = args;

        Ok(())
    }
}

impl MergeLuaAst for Arg {
    fn merge_lua_ast<'lua>(&mut self, table: LuaTable<'lua>) -> LuaResult<()> {
        self.id = NodeId::from_u32(table.get("id")?);
        self.pat.merge_lua_ast(table.get("pat")?)?;
        self.ty.merge_lua_ast(table.get("ty")?)?;

        Ok(())
    }
}

impl MergeLuaAst for P<Pat> {
    fn merge_lua_ast<'lua>(&mut self, table: LuaTable<'lua>) -> LuaResult<()> {
        let kind: LuaString = table.get("kind")?;

        self.node = match kind.to_str()? {
            "Wild" => PatKind::Wild,
            "Ident" => {
                let binding: LuaString = table.get("binding")?;
                let binding = match binding.to_str()? {
                    "ByRefImmutable" => BindingMode::ByRef(Immutable),
                    "ByRefMutable" => BindingMode::ByRef(Mutable),
                    "ByValueImmutable" => BindingMode::ByValue(Immutable),
                    "ByValueMutable" => BindingMode::ByValue(Mutable),
                    e => unimplemented!("Unknown ident binding: {}", e),
                };
                let ident: LuaString = table.get("ident")?;
                let ident = Ident::from_str(ident.to_str()?);

                // TODO: Sub-pattern
                PatKind::Ident(binding, ident, None)
            },
            "Tuple" => {
                let fragment_pos = table.get("fragment_pos")?;
                let lua_patterns: LuaTable = table.get("pats")?;
                let mut patterns = Vec::new();

                for lua_pat in lua_patterns.sequence_values::<LuaTable>() {
                    let mut pat = dummy_pat();

                    pat.merge_lua_ast(lua_pat?)?;
                    patterns.push(pat);
                }

                PatKind::Tuple(patterns, fragment_pos)
            },
            e => unimplemented!("MergeLuaAst unimplemented pat: {:?}", e),
        };

        Ok(())
    }
}

impl MergeLuaAst for Local {
    fn merge_lua_ast<'lua>(&mut self, table: LuaTable<'lua>) -> LuaResult<()> {
        // TODO: ty
        let pat: LuaTable = table.get("pat")?;
        let opt_init: Option<LuaTable> = table.get("init")?;

        self.pat.merge_lua_ast(pat)?;

        match &mut self.init {
            Some(existing_init) => {
                match opt_init {
                    Some(init) => existing_init.merge_lua_ast(init)?,
                    None => self.init = None,
                }
            },
            None => {
                if let Some(_) = opt_init {
                    unimplemented!("MergeLuaAst unimplemented local init update");
                }
            },
        }

        Ok(())
    }
}

impl MergeLuaAst for Crate {
    fn merge_lua_ast<'lua>(&mut self, table: LuaTable<'lua>) -> LuaResult<()> {
        self.module.merge_lua_ast(table.get("module")?)?;

        Ok(())
    }
}

impl MergeLuaAst for Mod {
    fn merge_lua_ast<'lua>(&mut self, table: LuaTable<'lua>) -> LuaResult<()> {
        let lua_items: LuaTable = table.get("items")?;

        self.inline = table.get("inline")?;

        // TODO: This may need to be improved if we want to delete or add
        // items as it currently expects items to be 1-1
        self.items.iter_mut().enumerate().map(|(i, item)| {
            let item_table: LuaTable = lua_items.get(i + 1)?;

            item.merge_lua_ast(item_table)
        }).collect()
    }
}

impl MergeLuaAst for P<Item> {
    fn merge_lua_ast<'lua>(&mut self, table: LuaTable<'lua>) -> LuaResult<()> {
        self.ident.name = Symbol::intern(&table.get::<_, String>("ident")?);

        // REVIEW: How to allow the type to be changed?
        match &mut self.node {
            ItemKind::Fn(fn_decl, _, _, block) => {
                let lua_fn_decl: LuaTable = table.get("decl")?;
                let lua_block: LuaTable = table.get("block")?;

                block.merge_lua_ast(lua_block)?;
                fn_decl.merge_lua_ast(lua_fn_decl)?;
            },
            ItemKind::Impl(.., items) => {
                let lua_items: LuaTable = table.get("items")?;

                // TODO: This may need to be improved if we want to delete or add
                // values as it currently expects values to be 1-1
                let res: LuaResult<Vec<()>> = items.iter_mut().enumerate().map(|(i, item)| {
                    let item_table: LuaTable = lua_items.get(i + 1)?;

                    item.merge_lua_ast(item_table)
                }).collect();

                res?;
            },
            ref e => warn!("MergeLuaAst unimplemented: {:?}", e),
        }

        Ok(())
    }
}

impl MergeLuaAst for P<Expr> {
    fn merge_lua_ast<'lua>(&mut self, table: LuaTable<'lua>) -> LuaResult<()> {
        let kind = table.get::<_, String>("kind")?;

        self.node = match kind.as_str() {
            "Path" => {
                let lua_segments: LuaTable = table.get("segments")?;
                let mut path = dummy_path();

                path.merge_lua_ast(lua_segments)?;

                // TODO: QSelf support
                ExprKind::Path(None, path)
            },
            "Lit" => {
                let val: LuaValue = table.get("value")?;
                let suffix: Option<String> = table.get("suffix")?;
                let lit = match val {
                    LuaValue::Boolean(val) => dummy_spanned(LitKind::Bool(val)),
                    LuaValue::Integer(i) => {
                        let suffix = match suffix.as_ref().map(AsRef::as_ref) {
                            None => LitIntType::Unsuffixed,
                            Some("u8") => LitIntType::Unsigned(UintTy::U8),
                            Some("u16") => LitIntType::Unsigned(UintTy::U16),
                            Some("u32") => LitIntType::Unsigned(UintTy::U32),
                            Some("u64") => LitIntType::Unsigned(UintTy::U64),
                            Some("u128") => LitIntType::Unsigned(UintTy::U128),
                            Some("usize") => LitIntType::Unsigned(UintTy::Usize),
                            Some("i8") => LitIntType::Signed(IntTy::I8),
                            Some("i16") => LitIntType::Signed(IntTy::I16),
                            Some("i32") => LitIntType::Signed(IntTy::I32),
                            Some("i64") => LitIntType::Signed(IntTy::I64),
                            Some("i128") => LitIntType::Signed(IntTy::I128),
                            Some("isize") => LitIntType::Signed(IntTy::Isize),
                            _ => unreachable!("Unknown int suffix"),
                        };

                        dummy_spanned(LitKind::Int(i as u128, suffix))
                    },
                    LuaValue::Number(num) => {
                        let mut string = num.to_string();

                        // to_string won't add a trailing period if unsuffixed
                        if !string.contains('.') {
                            string.push('.');
                        }

                        let sym = Symbol::intern(&string);

                        dummy_spanned(match suffix.as_ref().map(AsRef::as_ref) {
                            None => LitKind::FloatUnsuffixed(sym),
                            Some("f32") => LitKind::Float(sym, FloatTy::F32),
                            Some("f64") => LitKind::Float(sym, FloatTy::F64),
                            _ => unreachable!("Unknown float suffix"),
                        })
                    },
                    LuaValue::String(lua_string) => {
                        let is_bytes: bool = table.get("is_bytes")?;

                        if is_bytes {
                            let bytes: Vec<u8> = lua_string
                                .as_bytes()
                                .iter()
                                .map(|b| *b)
                                .collect();

                            dummy_spanned(LitKind::ByteStr(Rc::new(bytes)))
                        } else {
                            // TODO: Raw strings?
                            let symbol = Symbol::intern(lua_string.to_str()?);
                            let style = StrStyle::Cooked;

                            dummy_spanned(LitKind::Str(symbol, style))
                        }
                    },
                    LuaValue::Nil => {
                        let symbol = Symbol::intern("NIL");

                        dummy_spanned(LitKind::Err(symbol))
                    },
                    _ => unimplemented!("MergeLuaAst unimplemented lit: {:?}", val),
                };

                ExprKind::Lit(lit)
            },
            "Binary" | "AssignOp" | "Assign" => {
                let lua_lhs = table.get("lhs")?;
                let lua_rhs = table.get("rhs")?;
                let op: Option<String> = table.get("op")?;
                let op = op.map(|s| match s.as_str() {
                    "+" => BinOpKind::Add,
                    "-" => BinOpKind::Sub,
                    "*" => BinOpKind::Mul,
                    "/" => BinOpKind::Div,
                    "%" => BinOpKind::Rem,
                    "&&" => BinOpKind::And,
                    "||" => BinOpKind::Or,
                    "^" => BinOpKind::BitXor,
                    "&" => BinOpKind::BitAnd,
                    "|" => BinOpKind::BitOr,
                    "<<" => BinOpKind::Shl,
                    ">>" => BinOpKind::Shr,
                    "==" => BinOpKind::Eq,
                    "<" => BinOpKind::Lt,
                    "<=" => BinOpKind::Le,
                    "!=" => BinOpKind::Ne,
                    ">=" => BinOpKind::Ge,
                    ">" => BinOpKind::Gt,
                    e => unreachable!("Unknown BinOpKind: {}", e),
                });

                let mut lhs = dummy_expr();
                let mut rhs = dummy_expr();

                lhs.merge_lua_ast(lua_lhs)?;
                rhs.merge_lua_ast(lua_rhs)?;

                match kind.as_str() {
                    "Binary" => ExprKind::Binary(dummy_spanned(op.unwrap()), lhs, rhs),
                    "AssignOp" => ExprKind::AssignOp(dummy_spanned(op.unwrap()), lhs, rhs),
                    "Assign" => ExprKind::Assign(lhs, rhs),
                    _ => unreachable!(),
                }
            },
            "Array" => {
                let lua_exprs: LuaTable = table.get("values")?;
                let mut exprs = Vec::new();

                for lua_expr in lua_exprs.sequence_values::<LuaTable>() {
                    let mut expr = dummy_expr();

                    expr.merge_lua_ast(lua_expr?)?;

                    exprs.push(expr);
                }

                ExprKind::Array(exprs)
            },
            "Index" => {
                let lua_indexed: LuaTable = table.get("indexed")?;
                let lua_index: LuaTable = table.get("index")?;
                let mut indexed = dummy_expr();
                let mut index = dummy_expr();

                indexed.merge_lua_ast(lua_indexed)?;
                index.merge_lua_ast(lua_index)?;

                ExprKind::Index(indexed, index)
            },
            "Unary" => {
                let op: String = table.get("op")?;
                let lua_expr: LuaTable = table.get("expr")?;
                let op = match op.as_str() {
                    "*" => UnOp::Deref,
                    "!" => UnOp::Not,
                    "-" => UnOp::Neg,
                    e => unreachable!("Unknown UnOp: {}", e),
                };
                let mut expr = dummy_expr();

                expr.merge_lua_ast(lua_expr)?;

                ExprKind::Unary(op, expr)
            },
            "Call" => {
                let mut path = dummy_expr();
                let lua_path = table.get("path")?;
                let lua_args: LuaTable = table.get("args")?;
                let mut args = Vec::new();

                path.merge_lua_ast(lua_path)?;

                for lua_arg in lua_args.sequence_values::<LuaTable>() {
                    let mut arg = dummy_expr();

                    arg.merge_lua_ast(lua_arg?)?;
                    args.push(arg);
                }

                ExprKind::Call(path, args)
            },
            "MethodCall" => {
                let lua_args: LuaTable = table.get("args")?;
                let mut args = Vec::new();

                for lua_arg in lua_args.sequence_values::<LuaTable>() {
                    let mut arg = dummy_expr();

                    arg.merge_lua_ast(lua_arg?)?;
                    args.push(arg);
                }

                let name: String = table.get("name")?;
                let segment = PathSegment::from_ident(Ident::from_str(&name));

                ExprKind::MethodCall(segment, args)
            },
            "Field" => {
                let lua_expr: LuaTable = table.get("expr")?;
                let mut expr = dummy_expr();
                let name: String = table.get("name")?;

                expr.merge_lua_ast(lua_expr)?;

                ExprKind::Field(expr, Ident::from_str(&name))
            },
            "Ret" => {
                let opt_lua_expr: Option<LuaTable> = table.get("value")?;

                match opt_lua_expr {
                    Some(lua_expr) => {
                        let mut expr = dummy_expr();

                        expr.merge_lua_ast(lua_expr)?;

                        ExprKind::Ret(Some(expr))
                    },
                    None => ExprKind::Ret(None),
                }
            },
            "Tup" => {
                let lua_exprs: LuaTable = table.get("exprs")?;
                let mut exprs = Vec::new();

                for lua_expr in lua_exprs.sequence_values::<LuaTable>() {
                    let mut expr = dummy_expr();

                    expr.merge_lua_ast(lua_expr?)?;
                    exprs.push(expr);
                }

                ExprKind::Tup(exprs)
            },
            "AddrOf" => {
                let lua_expr = table.get("expr")?;
                let mut expr = dummy_expr();

                expr.merge_lua_ast(lua_expr)?;

                let mutability = match table.get::<_, String>("mutability")?.as_str() {
                    "Immutable" => Immutable,
                    "Mutable" => Mutable,
                    e => panic!("Found unknown addrof mutability: {}", e),
                };

                ExprKind::AddrOf(mutability, expr)
            },
            "Block" => {
                let lua_block = table.get("block")?;
                let mut block = dummy_block();
                let opt_label_str: Option<String> = table.get("label")?;
                let opt_label = opt_label_str.map(|s| Label { ident: Ident::from_str(&s) });

                block.merge_lua_ast(lua_block)?;

                ExprKind::Block(block, opt_label)
            },
            "While" => {
                let lua_cond = table.get("cond")?;
                let lua_block = table.get("block")?;
                let mut cond = dummy_expr();
                let mut block = dummy_block();
                let opt_label_str: Option<String> = table.get("label")?;
                let opt_label = opt_label_str.map(|s| Label { ident: Ident::from_str(&s) });

                block.merge_lua_ast(lua_block)?;
                cond.merge_lua_ast(lua_cond)?;

                ExprKind::While(cond, block, opt_label)
            },
            "If" => {
                let lua_cond = table.get("cond")?;
                let lua_then = table.get("then")?;
                let opt_lua_else: Option<_> = table.get("else")?;
                let mut cond = dummy_expr();
                let mut then = dummy_block();

                cond.merge_lua_ast(lua_cond)?;
                then.merge_lua_ast(lua_then)?;

                let opt_else = opt_lua_else.map(|lua_else| {
                    let mut expr = dummy_expr();

                    expr.merge_lua_ast(lua_else).map(|_| expr)
                }).transpose()?;

                ExprKind::If(cond, then, opt_else)
            },
            "Cast" => {
                let lua_expr = table.get("expr")?;
                let mut expr = dummy_expr();
                let lua_ty = table.get("ty")?;
                let mut ty = dummy_ty();

                expr.merge_lua_ast(lua_expr)?;
                ty.merge_lua_ast(lua_ty)?;

                ExprKind::Cast(expr, ty)
            },
            "Struct" => {
                let lua_path = table.get("path")?;
                let lua_fields: LuaTable = table.get("fields")?;
                let opt_lua_expr: Option<_> = table.get("expr")?;
                let opt_expr = opt_lua_expr.map(|lua_expr| {
                    let mut expr = dummy_expr();

                    expr.merge_lua_ast(lua_expr).map(|_| expr)
                }).transpose()?;
                let mut path = dummy_path();
                let mut fields = Vec::new();

                path.merge_lua_ast(lua_path)?;

                for field in lua_fields.sequence_values::<LuaTable>() {
                    let field = field?;
                    let string: String = field.get("ident")?;
                    let ident = Ident::from_str(&string);
                    let lua_expr = field.get("expr")?;
                    let mut expr = dummy_expr();
                    let is_shorthand = field.get("is_shorthand")?;

                    expr.merge_lua_ast(lua_expr)?;

                    fields.push(Field {
                        ident,
                        expr,
                        span: DUMMY_SP,
                        is_shorthand,
                        attrs: ThinVec::new(), // TODO
                    })
                }

                ExprKind::Struct(path, fields, opt_expr)
            },
            "Repeat" => {
                let lua_expr = table.get("expr")?;
                let lua_ac_expr = table.get("anon_const")?;
                let mut expr = dummy_expr();
                let mut ac_expr = dummy_expr();

                expr.merge_lua_ast(lua_expr)?;
                ac_expr.merge_lua_ast(lua_ac_expr)?;

                let anon_const = AnonConst {
                    id: DUMMY_NODE_ID,
                    value: ac_expr,
                };

                ExprKind::Repeat(expr, anon_const)
            },
            ref e => {
                warn!("MergeLuaAst unimplemented expr: {:?}", e);
                return Ok(());
            },
        };

        Ok(())
    }
}

impl MergeLuaAst for ImplItem {
    fn merge_lua_ast<'lua>(&mut self, table: LuaTable<'lua>) -> LuaResult<()> {
        self.ident.name = Symbol::intern(&table.get::<_, String>("ident")?);

        match &mut self.node {
            ImplItemKind::Method(sig, block) => {
                let lua_decl: LuaTable = table.get("decl")?;
                let lua_block: LuaTable = table.get("block")?;

                sig.decl.merge_lua_ast(lua_decl)?;
                block.merge_lua_ast(lua_block)?;

                // TODO: generics, attrs, ..
            },
            ref e => unimplemented!("MergeLuaAst for {:?}", e),
        }

        Ok(())
    }
}

impl MergeLuaAst for P<Ty> {
    fn merge_lua_ast<'lua>(&mut self, table: LuaTable<'lua>) -> LuaResult<()> {
        let kind: LuaString = table.get("kind")?;
        let kind = kind.to_str()?;

        self.node = match kind {
            "Path" => {
                let lua_segments: LuaTable = table.get("path")?;
                let mut path = dummy_path();

                path.merge_lua_ast(lua_segments)?;

                // TODO: QSelf support
                TyKind::Path(None, path)
            },
            "Ptr" | "Rptr" => {
                let mutbl = match table.get::<_, String>("mutbl")?.as_str() {
                    "Immutable" => Immutable,
                    "Mutable" => Mutable,
                    e => panic!("Found unknown ptr mutability: {}", e),
                };
                let lua_ty = table.get("ty")?;
                let mut ty = dummy_ty();

                ty.merge_lua_ast(lua_ty)?;

                let mut_ty = MutTy {ty, mutbl};

                if kind == "Ptr" {
                    TyKind::Ptr(mut_ty)
                } else {
                    let lua_lifetime: Option<LuaString> = table.get("lifetime")?;
                    let opt_lifetime = lua_lifetime.map(|s| {
                        s.to_str()
                            .map(|s| Ident::from_str(s))
                            .map(|i| Lifetime {id: DUMMY_NODE_ID, ident: i})
                    }).transpose()?;

                    TyKind::Rptr(opt_lifetime, mut_ty)
                }
            },
            "BareFn" => {
                let lua_decl = table.get("decl")?;
                let mut decl = dummy_fn_decl();
                let unsafety = match table.get::<_, String>("unsafety")?.as_str() {
                    "Unsafe" => Unsafety::Unsafe,
                    "Normal" => Unsafety::Normal,
                    e => panic!("Found unknown unsafety: {}", e),
                };
                let abi = match table.get::<_, String>("abi")?.as_str() {
                    "Cdecl" => Abi::Cdecl,
                    "C" => Abi::C,
                    "Rust" => Abi::Rust,
                    e => unimplemented!("Abi: {}", e),
                };

                decl.merge_lua_ast(lua_decl)?;

                let bare_fn = BareFnTy {
                    unsafety,
                    abi,
                    generic_params: Vec::new(), // TODO
                    decl,
                };

                TyKind::BareFn(P(bare_fn))
            },
            "Array" => {
                let lua_ty = table.get("ty")?;
                let lua_ac_expr = table.get("anon_const")?;
                let mut ac_expr = dummy_expr();
                let mut ty = dummy_ty();

                ac_expr.merge_lua_ast(lua_ac_expr)?;
                ty.merge_lua_ast(lua_ty)?;

                let anon_const = AnonConst {
                    id: DUMMY_NODE_ID,
                    value: ac_expr,
                };

                TyKind::Array(ty, anon_const)
            },
            "Typeof" => {
                let lua_ac_expr = table.get("anon_const")?;
                let mut ac_expr = dummy_expr();

                ac_expr.merge_lua_ast(lua_ac_expr)?;

                let anon_const = AnonConst {
                    id: DUMMY_NODE_ID,
                    value: ac_expr,
                };

                TyKind::Typeof(anon_const)
            },
            "Paren" | "Slice" => {
                let lua_ty = table.get("ty")?;
                let mut ty = dummy_ty();

                ty.merge_lua_ast(lua_ty)?;

                if kind == "Paren" {
                    TyKind::Paren(ty)
                } else {
                    TyKind::Slice(ty)
                }
            },
            "Tup" => {
                let lua_tys: LuaTable = table.get("tys")?;
                let mut tys = Vec::new();

                for lua_ty in lua_tys.sequence_values::<LuaTable>() {
                    let mut ty = dummy_ty();

                    ty.merge_lua_ast(lua_ty?)?;
                    tys.push(ty);
                }

                TyKind::Tup(tys)
            },
            "Never" => TyKind::Never,
            "ImplicitSelf" => TyKind::ImplicitSelf,
            "CVarArgs" => TyKind::CVarArgs,
            "Infer" => TyKind::Infer,
            ref e => unimplemented!("MergeLuaAst unimplemented ty kind: {:?}", e),
        };

        Ok(())
    }
}

impl MergeLuaAst for Path {
    fn merge_lua_ast<'lua>(&mut self, table: LuaTable<'lua>) -> LuaResult<()> {
        let mut segments = Vec::new();

        for lua_segment in table.sequence_values::<LuaTable>() {
            let lua_segment = lua_segment?;
            let ident: LuaString = lua_segment.get("ident")?;
            let lua_generics: Option<LuaTable> = lua_segment.get("generics")?;
            let mut path_segment = PathSegment::from_ident(Ident::from_str(ident.to_str()?));
            let opt_generics = lua_generics.map(|lua_generics| {
                let mut generics = dummy_generic_args();

                generics.merge_lua_ast(lua_generics).map(|_| P(generics))
            }).transpose()?;

            path_segment.args = opt_generics;

            segments.push(path_segment);
        }

        self.segments = segments;

        Ok(())
    }
}

impl MergeLuaAst for GenericArgs {
    fn merge_lua_ast<'lua>(&mut self, table: LuaTable<'lua>) -> LuaResult<()> {
        let kind: LuaString = table.get("kind")?;
        let kind = kind.to_str()?;

        *self = match kind {
            "AngleBracketed" => {
                let lua_args: LuaTable = table.get("args")?;
                let mut args = Vec::new();

                for lua_arg in lua_args.sequence_values::<LuaTable>() {
                    let mut arg = dummy_generic_arg();

                    arg.merge_lua_ast(lua_arg?)?;
                    args.push(arg);
                }

                GenericArgs::AngleBracketed(AngleBracketedArgs {
                    args,
                    span: DUMMY_SP,
                    bindings: Vec::new(), // TODO
                })
            },
            "Parenthesized" => unimplemented!("MergeLuaAst unimplemented for Parenthesized"),
            e => unimplemented!("Unknown GenericArgs kind: {}", e),
        };

        Ok(())
    }
}


impl MergeLuaAst for GenericArg {
    fn merge_lua_ast<'lua>(&mut self, table: LuaTable<'lua>) -> LuaResult<()> {
        let kind: LuaString = table.get("kind")?;
        let kind = kind.to_str()?;

        *self = match kind {
            "Type" => {
                let lua_ty = table.get("ty")?;
                let mut ty = dummy_ty();

                ty.merge_lua_ast(lua_ty)?;

                GenericArg::Type(ty)
            },
            e => unimplemented!("Unknown GenericArg kind: {}", e),
        };

        Ok(())
    }
}
