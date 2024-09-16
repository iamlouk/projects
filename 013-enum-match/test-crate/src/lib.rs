extern crate variantmatch;

#[cfg(test)]
mod tests {
    use super::*;
    use variantmatch::MatchVariants;

    #[derive(MatchVariants)]
    enum TestEnum {
        OptionA,
        OptionB(i32),
        OptionC(f64, i32)
    }

    #[test]
    fn test_match_variants() {
        let test_a1: TestEnum = TestEnum::OptionA;
        let test_a2: TestEnum = TestEnum::OptionA;
        let test_b1: TestEnum = TestEnum::OptionB(42);
        let test_b2: TestEnum = TestEnum::OptionB(1234);
        let test_c1: TestEnum = TestEnum::OptionC(3.14, 1000);
        let test_c2: TestEnum = TestEnum::OptionC(2.71, 321);

        assert!(test_a1.match_variants(&test_a2));
        assert!(test_b1.match_variants(&test_b2));
        assert!(test_c1.match_variants(&test_c2));

        assert!(!test_a2.match_variants(&test_b1));
        assert!(!test_b2.match_variants(&test_c1));
        assert!(!test_c2.match_variants(&test_a1));

    }
}
