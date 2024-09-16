mod lexer;
mod ast;
mod parser;

fn main() {
    println!("Hello, world!");

    let code = "
        let x = 2 in
        let y = 4 in 5 - 4 - 3 - 2 - 1
    ";
    let lexer = lexer::Lexer::new(0, code);
    let mut parser = parser::Parser::new(lexer);
    match parser.parse_expr() {
        Ok(ast) => {
            let mut buf = String::new();
            ast.to_string(&mut buf).unwrap();
            println!("{}", buf);
        },
        Err(e) => eprintln!("{:?}", e)
    }
}
