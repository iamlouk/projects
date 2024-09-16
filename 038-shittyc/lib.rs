#![feature(assert_matches)]

mod common;
mod lex;
mod ast;
mod codegen;

#[cfg(test)]
mod tests {
    use std::io::Write;
    use crate::{ast::Parser, codegen::CodeGen, lex::Lexer};

    fn prepare(name: &'static str, test_src: &'static str, main_src: &'static str) -> std::path::PathBuf {
        let mut dir = std::env::temp_dir();
        dir.push(name);
        dir.push(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs_f64().to_string());
        eprintln!("-> test directory: {:?}", &dir);
        std::fs::create_dir_all(&dir).expect("failed to create temp. dir");
        let mut main_dot_s = dir.clone();
        main_dot_s.push("main.s");
        let mut cmd = std::process::Command::new("riscv64-linux-gnu-gcc")
            .args(["-O1", "-x", "c", "-S", "-o", main_dot_s.to_str().unwrap(), "-"])
            .stdin(std::process::Stdio::piped())
            .spawn().expect("command spawning failed");
        cmd.stdin.as_ref().unwrap().write_all(main_src.as_bytes()).expect("failed to write to stdin");
        let status = cmd.wait().expect("main file compiler");
        assert!(status.success());

        let mut test_dot_s = dir.clone();
        test_dot_s.push("test.s");

        let buf = test_src.as_bytes().to_vec();
        let mut lex = Lexer::new(std::path::Path::new("text.c"), &buf);
        let mut p = Parser::new();
        let f = p.parse_function(&mut lex).unwrap();

        {
            let mut test_file = std::fs::File::create(&test_dot_s).expect("assembly dump file");
            {
                let mut cg = CodeGen::new(Box::new(&mut test_file));
                cg.header().expect("write header");
                cg.write(f.as_ref()).expect("write function");
            }
            test_file.flush().expect("flush");
        }

        let mut executable = dir.clone();
        executable.push("test.bin");
        let status = std::process::Command::new("riscv64-linux-gnu-gcc")
            .args([
                "-static", "-o", executable.to_str().unwrap(),
                main_dot_s.to_str().unwrap(), test_dot_s.to_str().unwrap()
            ])
            .spawn()
            .expect("command spawning failed").wait().expect("linker");
        assert!(status.success());
        executable
    }

    #[test]
    fn ret_zero() {
        let test_binary = prepare("ret_zero", "
            long zero() { return 0i64; }
            ", "
            #include <stdlib.h>
            #include <stdio.h>

            extern long zero();

            int main() { return zero() == 0 ? EXIT_SUCCESS : EXIT_FAILURE; }
            ");
        let status = std::process::Command::new("qemu-riscv64")
            .arg(test_binary).status().unwrap();
        assert!(status.success());
    }

    #[test]
    fn add() {
        let test_binary = prepare("add", "
            long add(long a, long b) { return a + b; }
            ", "
            #include <stdlib.h>
            #include <stdio.h>

            extern long add(long a, long b);

            int main() { return add(1, 2) == 3 ? EXIT_SUCCESS : EXIT_FAILURE; }
            ");
        let status = std::process::Command::new("qemu-riscv64")
            .arg(test_binary).status().unwrap();
        assert!(status.success());
    }

}
