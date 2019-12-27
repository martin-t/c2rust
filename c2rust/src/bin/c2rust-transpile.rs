#[macro_use]
extern crate clap;
extern crate c2rust_transpile;

use clap::{App, Values};
use regex::Regex;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use c2rust_transpile::{Diagnostic, ReplaceMode, TranspilerConfig};

fn main() {
    let yaml = load_yaml!("../transpile.yaml");
    let matches = App::from_yaml(yaml).get_matches();

    // Build a TranspilerConfig from the command line
    let cc_json_path = Path::new(matches.value_of("COMPILE_COMMANDS").unwrap());
    let cc_json_path = cc_json_path
        .canonicalize()
        .unwrap_or_else(|_| {
            panic!("Could not find compile_commands.json file at path: {}", cc_json_path.display())
        });
    let extra_args: Vec<&str> = match matches.values_of("extra-clang-args") {
        Some(args) => args.collect(),
        None => Vec::new(),
    };

    let enabled_warnings: HashSet<Diagnostic> = matches
        .values_of("warn")
        .unwrap_or_else(|| Values::default())
        .map(|s| Diagnostic::from_str(s).unwrap())
        .collect();

    let log_level = match matches.value_of("log-level") {
        Some("off") => log::LevelFilter::Off,
        Some("error") => log::LevelFilter::Error,
        Some("warn") => log::LevelFilter::Warn,
        Some("info") => log::LevelFilter::Info,
        Some("debug") => log::LevelFilter::Debug,
        Some("trace") => log::LevelFilter::Trace,
        _ => panic!("Invalid log level"),
    };

    let mut tcfg = TranspilerConfig {
        dump_untyped_context: matches.is_present("dump-untyped-clang-ast"),
        dump_typed_context: matches.is_present("dump-typed-clang-ast"),
        pretty_typed_context: matches.is_present("pretty-typed-clang-ast"),
        dump_function_cfgs: matches.is_present("dump-function-cfgs"),
        json_function_cfgs: matches.is_present("json-function-cfgs"),
        dump_cfg_liveness: matches.is_present("dump-cfgs-liveness"),
        dump_structures: matches.is_present("dump-structures"),
        debug_ast_exporter: matches.is_present("debug-ast-exporter"),
        verbose: matches.is_present("verbose"),

        incremental_relooper: !matches.is_present("no-incremental-relooper"),
        fail_on_error: matches.is_present("fail-on-error"),
        fail_on_multiple: matches.is_present("fail-on-multiple"),
        filter: {
            if matches.is_present("filter") {
                let filter = matches.value_of("filter").unwrap();
                Some(Regex::new(filter).unwrap())
            } else {
                None
            }
        },
        debug_relooper_labels: matches.is_present("debug-labels"),
        cross_checks: matches.is_present("cross-checks"),
        cross_check_backend: matches
            .value_of("cross-check-backend")
            .map(String::from)
            .unwrap(),
        cross_check_configs: matches
            .values_of("cross-check-config")
            .map(|vals| vals.map(String::from).collect::<Vec<_>>())
            .unwrap_or_default(),
        prefix_function_names: matches.value_of("prefix-function-names").map(String::from),

        // We used to guard asm translation with a command-line
        // option. Defaulting to enabled now, can add an option to disable if
        // needed.
        translate_asm: true,

        // We used to guard varargs with a command-line option before nightly
        // support landed. We may still want to disable this option to target
        // stable rust output.
        translate_valist: true,

        translate_const_macros: matches.is_present("translate-const-macros"),
        disable_refactoring: matches.is_present("disable-refactoring"),

        use_c_loop_info: !matches.is_present("ignore-c-loop-info"),
        use_c_multiple_info: !matches.is_present("ignore-c-multiple-info"),
        simplify_structures: !matches.is_present("no-simplify-structures"),
        overwrite_existing: matches.is_present("overwrite-existing"),
        reduce_type_annotations: matches.is_present("reduce-type-annotations"),
        reorganize_definitions: matches.is_present("reorganize-definitions"),
        emit_modules: matches.is_present("emit-modules"),
        emit_build_files: matches.is_present("emit-build-files"),
        output_dir: matches.value_of("output-dir").map(PathBuf::from),
        binaries: matches
            .values_of("binary")
            .map(|values| values.map(String::from).collect())
            .unwrap_or_else(|| vec![]),
        panic_on_translator_failure: {
            match matches.value_of("invalid-code") {
                Some("panic") => true,
                Some("compile_error") => false,
                _ => panic!("Invalid option"),
            }
        },
        replace_unsupported_decls: ReplaceMode::Extern,
        emit_no_std: matches.is_present("emit-no-std"),
        enabled_warnings,
        log_level,
    };
    // binaries imply emit-build-files
    if !tcfg.binaries.is_empty() {
        tcfg.emit_build_files = true
    };
    // emit-build-files implies emit-modules
    if tcfg.emit_build_files {
        tcfg.emit_modules = true
    };

    c2rust_transpile::transpile(tcfg, &cc_json_path, &extra_args);
}
