use proptest::prelude::*;
use proptest::test_runner::{Config, RngAlgorithm, TestRng, TestRunner};
use rlua_core::{FunctionProto, LuaValue};
use rlua_jit::JitConfig;

const PROPERTY_CASES: u32 = 64;

#[derive(Debug, Clone)]
pub enum ExprTree {
    Const(i64),
    Add(Box<ExprTree>, Box<ExprTree>),
    Sub(Box<ExprTree>, Box<ExprTree>),
    Mul(Box<ExprTree>, Box<ExprTree>),
}

impl ExprTree {
    pub fn render(&self) -> String {
        match self {
            Self::Const(value) => format!("({value})"),
            Self::Add(left, right) => format!("({} + {})", left.render(), right.render()),
            Self::Sub(left, right) => format!("({} - {})", left.render(), right.render()),
            Self::Mul(left, right) => format!("({} * {})", left.render(), right.render()),
        }
    }

    pub fn eval(&self) -> i64 {
        match self {
            Self::Const(value) => *value,
            Self::Add(left, right) => left.eval() + right.eval(),
            Self::Sub(left, right) => left.eval() - right.eval(),
            Self::Mul(left, right) => left.eval() * right.eval(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtoSnapshot {
    num_params: u8,
    is_vararg: bool,
    max_stack_size: u8,
    code: Vec<u32>,
    constants: Vec<String>,
    prototypes: Vec<ProtoSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeExpectation {
    pub sum: i64,
    pub diff: i64,
    pub product: i64,
}

pub fn deterministic_runner() -> TestRunner {
    let config = Config {
        cases: PROPERTY_CASES,
        failure_persistence: None,
        ..Config::default()
    };
    let rng = TestRng::from_seed(RngAlgorithm::ChaCha, &[0x5a; 32]);
    TestRunner::new_with_rng(config, rng)
}

pub fn expr_strategy() -> impl Strategy<Value = ExprTree> {
    let leaf = (-12i64..=12).prop_map(ExprTree::Const);
    leaf.prop_recursive(4, 64, 2, |inner| {
        prop_oneof![
            (inner.clone(), inner.clone())
                .prop_map(|(left, right)| ExprTree::Add(Box::new(left), Box::new(right))),
            (inner.clone(), inner.clone())
                .prop_map(|(left, right)| ExprTree::Sub(Box::new(left), Box::new(right))),
            (inner.clone(), inner.clone())
                .prop_map(|(left, right)| ExprTree::Mul(Box::new(left), Box::new(right))),
        ]
    })
}

pub fn expr_pair_strategy() -> impl Strategy<Value = (ExprTree, ExprTree)> {
    (expr_strategy(), expr_strategy())
}

pub fn return_program(expr: &ExprTree) -> String {
    format!("return {}\n", expr.render())
}

pub fn locals_program(left: &ExprTree, right: &ExprTree) -> (String, RuntimeExpectation) {
    let source = format!(
        "local left = {}\nlocal right = {}\nreturn left + right, left - right, left * right\n",
        left.render(),
        right.render()
    );
    let expectation = RuntimeExpectation {
        sum: left.eval() + right.eval(),
        diff: left.eval() - right.eval(),
        product: left.eval() * right.eval(),
    };
    (source, expectation)
}

pub fn snapshot_proto(proto: &FunctionProto) -> ProtoSnapshot {
    ProtoSnapshot {
        num_params: proto.num_params,
        is_vararg: proto.is_vararg,
        max_stack_size: proto.max_stack_size,
        code: proto.code.iter().map(|instruction| instruction.0).collect(),
        constants: proto.constants.iter().map(snapshot_value).collect(),
        prototypes: proto.prototypes.iter().map(snapshot_proto).collect(),
    }
}

pub fn execute_program(source: &str) -> Result<Vec<LuaValue>, String> {
    let proto = rlua_compiler::compile_source(source).map_err(|error| error.to_string())?;
    let mut state = rlua_vm::VmState::with_jit_config(JitConfig {
        enabled: false,
        ..JitConfig::default()
    });
    rlua_stdlib::register_stdlib(&mut state);
    rlua_vm::execute(&mut state, proto).map_err(|error| error.to_string())
}

pub fn value_as_i64(value: &LuaValue) -> Option<i64> {
    match value {
        LuaValue::Number(number)
            if number.fract() == 0.0
                && *number >= i64::MIN as f64
                && *number <= i64::MAX as f64 =>
        {
            Some(*number as i64)
        }
        _ => None,
    }
}

fn snapshot_value(value: &LuaValue) -> String {
    match value {
        LuaValue::Table(_) => "table".to_owned(),
        LuaValue::Function(_) => "function".to_owned(),
        LuaValue::Thread(_) => "thread".to_owned(),
        _ => value.to_lua_string(),
    }
}
