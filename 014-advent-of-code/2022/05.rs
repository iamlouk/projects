


fn main() -> std::io::Result<()> {
    println!("Hello World!");

    let mut stacks: Vec<Vec<char>> = vec![];

    let mut buffer = String::new();

    loop {
        buffer.clear();
        std::io::stdin().read_line(&mut buffer)?;
        let chars: Vec<char> = buffer.chars().collect();
        assert!(chars.len() > 0 && chars[chars.len() - 1] == '\n');
        println!("line: {:?}", buffer);
        if chars.len() == 1 {
            break;
        }

        if chars[0] == ' ' && chars[1] == '1' {
            buffer.clear();
            std::io::stdin().read_line(&mut buffer)?;
            assert!(buffer == "\n");
            break;
        } 

        let mut i = 0;
        let mut pos = 1;
        while pos + 1 < chars.len() {
            if chars[pos] == ' ' {
                i += 1;
                pos += 4;
                continue;
            }

            assert!(chars[pos-1] == '[' && chars[pos+1] == ']');
            let item: char = chars[pos];
            while stacks.len() <= i {
                stacks.push(vec![]);
            }

            stacks[i].insert(0, item);
            i += 1;
            pos += 4;
        }
    }

    println!("stacks: {:?}", stacks);    

    loop {
        buffer.clear();
        std::io::stdin().read_line(&mut buffer)?;
        if buffer == "" || buffer == "\n" {
            break;
        }

        let mut lineiter = buffer.split(" ");
        assert!(lineiter.next() == Some("move"));
        let num = lineiter.next().unwrap().parse::<usize>().unwrap();
        assert!(lineiter.next() == Some("from"));
        let src = lineiter.next().unwrap().parse::<usize>().unwrap() - 1;
        assert!(lineiter.next() == Some("to"));
        let dst = lineiter.next().unwrap().trim().parse::<usize>().unwrap() -1;

        for _ in 0..num {
            let item = stacks[src].pop().unwrap();
            stacks[dst].push(item);
        }
    }

    println!("stacks: {:?}", stacks);

    let mut tos = String::with_capacity(stacks.len());
    for stack in stacks {
        tos.push(stack[stack.len() - 1]);
    }
    println!("tos: {:?}", tos);
    Ok(())
}
