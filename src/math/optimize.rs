use crate::economy::GOOD_TYPE_COUNT;

/// Port of Java's calculateOutputMaximizingInputsAnalyticalWithMarketPrices
///
/// For Cobb-Douglas: U = A * product(x_i ^ alpha_i)
/// Subject to: sum(p_i * x_i) = budget
///
/// Optimal x_i = (alpha_i / sum(alphas)) * (budget / p_i)
pub fn calculate_optimal_basket(
    budget: f64,
    prices: &[Option<f64>; GOOD_TYPE_COUNT],
    exponents: &[f64; GOOD_TYPE_COUNT],
) -> [f64; GOOD_TYPE_COUNT] {
    let mut optimal_volumes = [0.0; GOOD_TYPE_COUNT];

    let mut total_alpha = 0.0;
    for (i, &alpha) in exponents.iter().enumerate() {
        if prices[i].is_some() && alpha > 0.0 {
            total_alpha += alpha;
        }
    }

    if total_alpha == 0.0 || budget <= 0.0 {
        return optimal_volumes;
    }

    for i in 0..GOOD_TYPE_COUNT {
        if let Some(price) = prices[i] {
            if price > 0.0 && exponents[i] > 0.0 {
                optimal_volumes[i] = (exponents[i] / total_alpha) * (budget / price);
            }
        }
    }

    optimal_volumes
}

/// CES function implementation for budget optimization
/// U = (sum(alpha_i * x_i ^ rho)) ^ (1/rho)
/// Subject to sum(p_i * x_i) = budget
///
/// Optimal x_i = budget * ( (alpha_i / p_i) ^ (1 / (1-rho)) ) / sum( p_j * (alpha_j / p_j) ^ (1 / (1-rho)) )
pub fn calculate_optimal_basket_ces(
    budget: f64,
    prices: &[Option<f64>; GOOD_TYPE_COUNT],
    exponents: &[f64; GOOD_TYPE_COUNT],
    rho: f64,
) -> [f64; GOOD_TYPE_COUNT] {
    let mut optimal_volumes = [0.0; GOOD_TYPE_COUNT];

    if budget <= 0.0 || rho == 0.0 || rho >= 1.0 {
        // Fallback or error case. rho should be < 1 and != 0 for standard CES.
        // If rho -> 0, it's Cobb-Douglas.
        if rho == 0.0 {
            return calculate_optimal_basket(budget, prices, exponents);
        }
        return optimal_volumes;
    }

    let s = 1.0 / (1.0 - rho);
    let mut denominator = 0.0;

    let mut terms = [0.0; GOOD_TYPE_COUNT];
    for i in 0..GOOD_TYPE_COUNT {
        if let Some(price) = prices[i] {
            if price > 0.0 && exponents[i] > 0.0 {
                let term = (exponents[i] / price).powf(s);
                terms[i] = term;
                denominator += price * term;
            }
        }
    }

    if denominator == 0.0 {
        return optimal_volumes;
    }

    for i in 0..GOOD_TYPE_COUNT {
        if terms[i] > 0.0 {
            optimal_volumes[i] = budget * terms[i] / denominator;
        }
    }

    optimal_volumes
}
