#![feature(assert_matches)]

mod ast;
mod codegen;
mod common;
mod lex;

#[cfg(test)]
mod tests {
    use crate::{ast::Parser, codegen::CodeGen, lex::Lexer};
    use std::io::Write;

    fn prepare(
        name: &'static str,
        test_src: &'static str,
        main_src: &'static str,
    ) -> std::path::PathBuf {
        let mut dir = std::env::temp_dir();
        dir.push(name);
        dir.push(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs_f64()
                .to_string(),
        );
        eprintln!("-> test directory: {:?}", &dir);
        std::fs::create_dir_all(&dir).expect("failed to create temp. dir");
        let mut main_dot_s = dir.clone();
        main_dot_s.push("main.s");
        let mut cmd = std::process::Command::new("riscv64-linux-gnu-gcc")
            .args([
                "-O1",
                "-x",
                "c",
                "-S",
                "-o",
                main_dot_s.to_str().unwrap(),
                "-",
            ])
            .stdin(std::process::Stdio::piped())
            .spawn()
            .expect("command spawning failed");
        cmd.stdin
            .as_ref()
            .unwrap()
            .write_all(main_src.as_bytes())
            .expect("failed to write to stdin");
        let status = cmd.wait().expect("main file compiler");
        assert!(status.success());

        let mut test_dot_s = dir.clone();
        test_dot_s.push("test.s");

        let buf = test_src.as_bytes().to_vec();
        let mut lex = Lexer::new(std::path::Path::new("text.c"), &buf);
        let mut p = Parser::new();

        {
            let mut test_file = std::fs::File::create(&test_dot_s).expect("assembly dump file");
            {
                let mut cg = CodeGen::new(Box::new(&mut test_file));
                cg.header().expect("write header");
                while let Some(f) = p.parse_function(&mut lex).unwrap() {
                    cg.write(f.as_ref()).expect("write function");
                }
            }
            test_file.flush().expect("flush");
        }

        let mut executable = dir.clone();
        executable.push("test.bin");
        let status = std::process::Command::new("riscv64-linux-gnu-gcc")
            .args([
                "-static",
                "-o",
                executable.to_str().unwrap(),
                main_dot_s.to_str().unwrap(),
                test_dot_s.to_str().unwrap(),
            ])
            .spawn()
            .expect("command spawning failed")
            .wait()
            .expect("linker");
        assert!(status.success());
        executable
    }

    #[test]
    fn ret_zero() {
        let test_binary = prepare(
            "ret_zero",
            "
            long zero() { return 0i64; }
            ",
            "
            #include <stdlib.h>

            extern long zero();

            int main() { return zero() == 0 ? EXIT_SUCCESS : EXIT_FAILURE; }
            ",
        );
        let status = std::process::Command::new("qemu-riscv64")
            .arg(test_binary)
            .status()
            .unwrap();
        assert!(status.success());
    }

    #[test]
    fn add() {
        let test_binary = prepare(
            "add",
            "
            long add(long a, long b) { return a + b; }
            ",
            "
            #include <stdlib.h>

            extern long add(long a, long b);

            int main() { return add(1, 2) == 3 ? EXIT_SUCCESS : EXIT_FAILURE; }
            ",
        );
        let status = std::process::Command::new("qemu-riscv64")
            .arg(test_binary)
            .status()
            .unwrap();
        assert!(status.success());
    }

    #[test]
    fn if_else() {
        let test_binary = prepare(
            "if-else",
            "
            long min(long a, long b) {
              if (a < b)
                return a;
              return b;
            }
            ",
            "
            #include <stdlib.h>
            #include <assert.h>

            long min(long a, long b);

            int main() {
              assert(min(1, 2) == 1);
              assert(min(6, 5) == 5);
              return EXIT_SUCCESS;
            }",
        );
        let status = std::process::Command::new("qemu-riscv64")
            .arg(test_binary)
            .status()
            .unwrap();
        assert!(status.success());
    }

    #[test]
    fn fibs() {
        let test_binary = prepare(
            "fibs",
            "
            long fib(long n) {
              long a = 0i64, b = 1i64, i, tmp;
              for (i = 0i64; i < n; i = i + 1i64) {
                tmp = a;
                a = a + b;
                b = tmp;
              }
              return a;
            }
            ",
            "
            #include <stdlib.h>
            #include <assert.h>

            long fib(long n);

            int main() {
              assert(fib(0) == 0);
              assert(fib(1) == 1);
              assert(fib(2) == 1);
              assert(fib(3) == 2);
              assert(fib(4) == 3);
              assert(fib(5) == 5);
              assert(fib(6) == 8);
              assert(fib(7) == 13);
              assert(fib(8) == 21);
              return EXIT_SUCCESS;
            }",
        );
        let status = std::process::Command::new("qemu-riscv64")
            .arg(test_binary)
            .status()
            .unwrap();
        assert!(status.success());
    }
}
