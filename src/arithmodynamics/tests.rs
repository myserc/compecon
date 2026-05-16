#[cfg(test)]
mod tests {
    use crate::arithmodynamics::*;

    #[test]
    fn test_prime_sieve() {
        assert_eq!(get_prime_value_for_counts(1), 2);
        assert_eq!(get_prime_value_for_counts(2), 3);
        assert_eq!(get_prime_value_for_counts(3), 5);
    }

    #[test]
    fn test_counts_mapping() {
        assert_eq!(get_counts_for_prime_value(2), 1);
        assert_eq!(get_counts_for_prime_value(3), 2);
        assert_eq!(get_counts_for_prime_value(4), 2);
        assert_eq!(get_counts_for_prime_value(5), 3);
    }

    #[test]
    fn test_gpf() {
        assert_eq!(greatest_prime_factor(10), 5);
        assert_eq!(greatest_prime_factor(15), 5);
        assert_eq!(greatest_prime_factor(28), 7);
        assert_eq!(greatest_prime_factor(13), 13);
    }
}
