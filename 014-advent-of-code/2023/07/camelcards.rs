use std::io;

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Clone, Copy)]
enum Kind {
    FiveOfAKind,
    FourOfAKind,
    FullHouse,
    ThreeOfAKind,
    TwoPair,
    OnePair,
    HighCard
}

#[derive(Debug, PartialEq)]
struct Cards {
    kind: Kind,
    cards: [i32; 5]
}

impl std::cmp::PartialOrd for Cards {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        use std::cmp::Ordering;
        let ord = self.kind.cmp(&other.kind);
        if ord != Ordering::Equal {
            return Some(ord);
        }
        return Some(other.cards.cmp(&self.cards));
    }
}

fn classify(cards: Vec<i32>) -> Cards {
    assert!(cards.len() == 5);
    let kind = Kind::HighCard;
    let cards = [cards[0], cards[1], cards[2], cards[3], cards[4]];
    // There surely is a better solution, but this works:
    for (pos, card) in cards.iter().enumerate() {
        let nsame = cards.iter().enumerate().filter(|(i, c)| *i != pos && *c == card).count();
        if nsame == 4 {
            return Cards { kind: Kind::FiveOfAKind, cards };
        }
        if nsame == 3 {
            return Cards { kind: Kind::FourOfAKind, cards };
        }

        let mut others = cards.iter().filter(|c| *c != card).collect::<Vec<_>>();
        others.sort_unstable();
        others.dedup();
        if nsame == 2 {
            assert!(1 <= others.len() && others.len() <= 2);
            if others.len() == 1 {
                return Cards { kind: Kind::FullHouse, cards };
            } else {
                return Cards { kind: Kind::ThreeOfAKind, cards };
            }
        }

        if nsame == 1 && others.len() == 1 {
            return Cards { kind: Kind::FullHouse, cards };
        }

        if nsame == 1 && others.len() == 2 {
            return Cards { kind: Kind::TwoPair, cards };
        }

        if nsame == 1 && others.len() == 3 {
            return Cards { kind: Kind::OnePair, cards };
        }
    }

    return Cards { kind, cards };
}

fn main() {
    let mut hands = io::stdin().lines()
        .map(|x| x.unwrap())
        .map(|line|
            (line.split(' ').nth(1).unwrap().trim().parse::<u64>().unwrap(),
             classify(line
                .split(' ').nth(0).unwrap().trim().chars()
                .map(|x| match x {
                    '2' => 0, '3' => 1, '4' => 2,
                    '5' => 3, '6' => 4, '7' => 5,
                    '8' => 6, '9' => 7, 'T' => 8,
                    'J' => 9, 'Q' => 10, 'K' => 11,
                    'A' => 12, _ => panic!("unexpected card")
                })
                .collect::<Vec<_>>())))
        .collect::<Vec<_>>();
    hands.sort_unstable_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

    /*
    for hand in hands.iter() {
        let kind = hand.1.kind;
        let cards = hand.1.cards
            .map(|x| match x {
                0 => '2', 1 => '3', 2 => '4', 3 => '5', 4 => '6', 5 => '7', 6 => '8',
                7 => '9', 8 => 'T', 9 => 'J', 10 => 'Q', 11 => 'K', 12 => 'A',
                _ => panic!()
            });
        println!("cards: {cards:?}, kind: {kind:?}");
    }
    */

    let winnings: u64 = hands
        .iter().enumerate()
        .map(|(i, (bid, _))| ((hands.len() - i) as u64) * *bid)
        .sum();

    println!("winnings: {winnings}");
}

