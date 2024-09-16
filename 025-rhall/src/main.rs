#![feature(assert_matches)]
#![allow(clippy::type_complexity)]

use std::io::Read;
use std::rc::Rc;

mod ast;
mod core;
mod eval;
mod lex;

use eval::Env;

use crate::ast::Parser;
use crate::lex::Lexer;

fn main() {
    let mut buf = String::new();
    std::io::stdin()
        .read_to_string(&mut buf)
        .expect("I/O failure");

    let mut env = Env::new();
    add_builtins(&mut env);
    let mut lexer = Lexer::new(buf.as_str(), 0, &mut env.string_pool);
    let mut parser = Parser::new(&mut lexer);
    let mut node = match parser.parse_all() {
        Ok(node) => node,
        Err(e) => {
            eprintln!("parsing failed: {:?}", e);
            std::process::exit(1)
        }
    };
    println!("# AST: {}", node.as_ref());
    let typ = match node.typecheck(&mut env, None) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("type-check failed: {:?}", e);
            std::process::exit(1)
        }
    };
    println!("# TYP: {}", typ);

    match env.eval(node.as_ref()) {
        Ok(val) => println!("{}", val),
        Err(e) => eprintln!("{:?}", e),
    };
}

fn add_builtins(env: &mut Env) {
    use crate::core::{Builtin, Type, TypeParam, Value};
    let t1_str: Rc<str> = Rc::from("A");
    let x_str: Rc<str> = Rc::from("x");

    env.add_global(
        "Process/exit",
        Value::Builtin(Rc::new(Builtin {
            name: "Process/exit",
            argtypes: vec![(Rc::from("code"), env.int_type.clone())],
            rettyp: env.int_type.clone(),
            f: Box::new(|args| {
                let code = match args[0] {
                    Value::Int(x) => x,
                    _ => panic!(),
                };
                std::process::exit(code as i32)
            }),
        })),
    );

    let t1tp = TypeParam {
        name: t1_str.clone(),
        id: line!() as u64,
    };
    let t1 = Rc::new(Type::Generic(t1tp.clone()));
    env.add_global(
        "Option",
        Value::Builtin(Rc::new(Builtin {
            name: "Option",
            argtypes: vec![(t1_str.clone(), Rc::new(Type::Type(Some(t1tp), None)))],
            rettyp: Rc::new(Type::Type(None, Some(Rc::new(Type::Option(t1))))),
            f: Box::new(|args| {
                let t = match &args[0] {
                    Value::Type(t) => t.clone(),
                    _ => panic!(),
                };
                Ok(Value::Type(Rc::new(Type::Option(t))))
            }),
        })),
    );

    let t1tp = TypeParam {
        name: t1_str.clone(),
        id: line!() as u64,
    };
    let t1 = Rc::new(Type::Generic(t1tp.clone()));
    env.add_global(
        "None",
        Value::Builtin(Rc::new(Builtin {
            name: "Some",
            argtypes: vec![(t1_str.clone(), Rc::new(Type::Type(Some(t1tp), None)))],
            rettyp: Rc::new(Type::Option(t1)),
            f: Box::new(|args| {
                let t = match &args[0] {
                    Value::Type(t) => t.clone(),
                    _ => panic!(),
                };
                Ok(Value::Option(t, None))
            }),
        })),
    );

    /*
    let t1tp = TypeParam { name: t1_str.clone(), id: line!() as u64 };
    let t1 = Rc::new(Type::Generic(t1tp.clone()));
    env.add_global("None", Value::Builtin(Rc::new(Builtin {
        name: "None",
        argtypes: vec![(t1_str.clone(), Rc::new(Type::Type(Some(t1tp), None)))],
        rettyp: Rc::new(Type::Option(t1)),
        f: Box::new(|args| {
            todo!()
        })
    })));
    */
}

/*
fn add_builtins(env: &mut Env) {
    use crate::core::{Value, Builtin, Type};
    let t1_str: Rc<str> = Rc::from("A");
    let t2_str: Rc<str> = Rc::from("B");
    let f_str: Rc<str> = Rc::from("f");
    let default_str: Rc<str> = Rc::from("default");


    let t = Rc::new(Type::Generic(t1_str.clone(), line!() as u64));
    env.add_global("Option", Value::Builtin(Rc::new(Builtin {
        name: "Option",
        argtypes: vec![(x_str.clone(), Rc::new(Type::Type(Some(t.clone()))))],
        rettyp: Rc::new(Type::Option(t)),
        f: Box::new(|args| {
            Ok(Value::Type(Rc::new(Type::Option(match &args[0] {
                Value::Type(t) => t.clone(),
                _ => panic!()
            }))))
        })
    })));

    let t = Rc::new(Type::Generic(t1_str.clone(), line!() as u64));
    env.add_global("None", Value::Builtin(Rc::new(Builtin {
        name: "None",
        argtypes: vec![(x_str.clone(), Rc::new(Type::Type(Some(t.clone()))))],
        rettyp: Rc::new(Type::Option(t.clone())),
        f: Box::new(|args| {
            let t = match &args[0] { Value::Type(t) => t.clone(), _ => panic!() };
            Ok(Value::Option(t, None))
        })
    })));

    {
        let x_str = x_str.clone();
        let t = Rc::new(Type::Generic(t1_str.clone(), line!() as u64));
        env.add_global("Some", Value::Builtin(Rc::new(Builtin {
            name: "Some",
            argtypes: vec![(t1_str.clone(), Rc::new(Type::Type(Some(t.clone()))))],
            rettyp: Rc::new(Type::Lambda(vec![(x_str.clone(), t.clone())], Rc::new(Type::Option(t.clone())))),
            f: Box::new(move |args| {
                let t = match &args[0] { Value::Type(t) => t.clone(), _ => panic!() };
                Ok(Value::Builtin(Rc::new(Builtin {
                    name: "Some!",
                    argtypes: vec![(x_str.clone(), t.clone())],
                    rettyp: Rc::new(Type::Option(t.clone())),
                    f: Box::new(move |args| {
                        Ok(Value::Option(t.clone(), Some(Box::new(args[0].clone()))))
                    })
                })))
            })
        })));
    }

    _ = t1_str;
    _ = t2_str;
    _ = x_str;
    _ = f_str;
}
*/

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn examples() {
        let mut env = Env::new();
        add_builtins(&mut env);

        use std::path::PathBuf;
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("examples");
        for entry in std::fs::read_dir(&path).unwrap() {
            let (expected, name) = match entry {
                Ok(entry) if entry.file_name().to_str().unwrap().ends_with(".expected") => (
                    std::fs::read_to_string(entry.path()).unwrap(),
                    entry.file_name().into_string().unwrap(),
                ),
                Ok(_) => continue,
                Err(e) => panic!("{:?}", e),
            };
            let name = name.strip_suffix(".expected").unwrap();
            path.push(name.to_string() + ".dhall");
            let sourcecode = std::fs::read_to_string(&path).unwrap();
            eprintln!("running example: {}", name);

            let mut lexer = Lexer::new(sourcecode.as_str(), 0, &mut env.string_pool);
            let mut parser = Parser::new(&mut lexer);
            let mut example = parser.parse_all().expect("parsing failed");

            example
                .typecheck(&mut env, None)
                .expect("type check failed");

            let res = format!("{}", env.eval(&example).expect("evaluation failed"));
            assert_eq!(expected.trim(), res.trim());

            path.pop();
        }
    }
}
