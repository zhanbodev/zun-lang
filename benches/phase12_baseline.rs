use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use zun::interpreter::HttpRoute;
use zun::lexer::Lexer;
use zun::parse;
use zun::runtime::{
    HandlerInput, ProgramState, RuntimeContext, RuntimeMode, execute_http_route,
    execute_program_with_writer, execute_source_with_writer, load_context_and_program,
};

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("benches")
        .join("fixtures")
}

fn read_fixture(relative: &str) -> String {
    fs::read_to_string(fixtures_dir().join(relative)).expect("benchmark fixture should exist")
}

fn benchmark_lexer(c: &mut Criterion) {
    let source = read_fixture("lexer_parser_small.zun");
    let mut group = c.benchmark_group("frontend");
    group.bench_with_input(
        BenchmarkId::new("lexer", "small_script"),
        &source,
        |b, source| {
            b.iter(|| {
                let mut lexer = Lexer::new(black_box(source));
                let tokens = lexer.lex_all().expect("lexing should succeed");
                black_box(tokens.len());
            });
        },
    );
    group.finish();
}

fn benchmark_parser(c: &mut Criterion) {
    let source = read_fixture("lexer_parser_small.zun");
    let mut group = c.benchmark_group("frontend");
    group.bench_with_input(
        BenchmarkId::new("parser", "small_script"),
        &source,
        |b, source| {
            b.iter(|| {
                let ast = parse(black_box(source)).expect("parsing should succeed");
                black_box(ast.len());
            });
        },
    );
    group.finish();
}

fn benchmark_runtime_expression(c: &mut Criterion) {
    let source = read_fixture("runtime_expression.zun");
    let project_root = fixtures_dir();
    let mut group = c.benchmark_group("runtime");
    group.bench_with_input(
        BenchmarkId::new("execute_expression", "small_script"),
        &source,
        |b, source| {
            b.iter(|| {
                let mut state = ProgramState::new();
                let mut context = RuntimeContext::new(RuntimeMode::Test, project_root.clone());
                let mut output = Vec::new();
                execute_source_with_writer(
                    black_box(source),
                    &mut state,
                    &mut context,
                    &mut output,
                )
                .expect("execution should succeed");
                black_box(state.env.len());
            });
        },
    );
    group.finish();
}

fn benchmark_function_call(c: &mut Criterion) {
    let source = read_fixture("function_call.zun");
    let project_root = fixtures_dir();
    let mut group = c.benchmark_group("runtime");
    group.bench_with_input(
        BenchmarkId::new("function_call", "single_definition_and_call"),
        &source,
        |b, source| {
            b.iter(|| {
                let mut state = ProgramState::new();
                let mut context = RuntimeContext::new(RuntimeMode::Test, project_root.clone());
                let mut output = Vec::new();
                execute_source_with_writer(
                    black_box(source),
                    &mut state,
                    &mut context,
                    &mut output,
                )
                .expect("execution should succeed");
                black_box(state.env.len());
            });
        },
    );
    group.finish();
}

fn benchmark_module_load(c: &mut Criterion) {
    let main_path = fixtures_dir().join("modules").join("main.zun");
    let mut group = c.benchmark_group("runtime");
    group.bench_with_input(
        BenchmarkId::new("module_load", "relative_link"),
        &main_path,
        |b, path| {
            b.iter(|| {
                let (mut context, program) =
                    load_context_and_program(RuntimeMode::Test, black_box(path.as_path()))
                        .expect("program should load");
                let mut state = ProgramState::new();
                let mut output = Vec::new();
                execute_program_with_writer(
                    &program.statements,
                    &mut state,
                    &mut context,
                    &mut output,
                )
                .expect("execution should succeed");
                black_box(state.env.len());
            });
        },
    );
    group.finish();
}

fn load_http_route(path: &Path) -> (RuntimeContext, HttpRoute) {
    let (mut context, program) =
        load_context_and_program(RuntimeMode::Test, path).expect("route fixture should load");
    let mut state = ProgramState::new();
    let mut output = Vec::new();
    execute_program_with_writer(&program.statements, &mut state, &mut context, &mut output)
        .expect("route fixture should execute");
    let route = context
        .routes()
        .first()
        .cloned()
        .expect("route should be registered");
    (context, route)
}

fn benchmark_http_handler(c: &mut Criterion) {
    let route_path = fixtures_dir().join("http").join("health_route.zun");
    let (context, route) = load_http_route(&route_path);
    let input = HandlerInput::new("/health");
    let mut group = c.benchmark_group("runtime");
    group.bench_function("http_handler/minimal_health", |b| {
        b.iter(|| {
            let result = execute_http_route(black_box(&route), black_box(&input), &context);
            black_box(result);
        });
    });
    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .sample_size(10)
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(1))
        .without_plots();
    targets =
        benchmark_lexer,
        benchmark_parser,
        benchmark_runtime_expression,
        benchmark_function_call,
        benchmark_module_load,
        benchmark_http_handler
}
criterion_main!(benches);
