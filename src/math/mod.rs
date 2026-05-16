pub fn cobb_douglas(inputs: &[f64], alphas: &[f64], total_factor_productivity: f64) -> f64 {
    let mut result = total_factor_productivity;
    for (input, alpha) in inputs.iter().zip(alphas.iter()) {
        result *= input.powf(*alpha);
    }
    result
}

pub fn ces(inputs: &[f64], alphas: &[f64], rho: f64, total_factor_productivity: f64) -> f64 {
    let mut sum = 0.0;
    for (input, alpha) in inputs.iter().zip(alphas.iter()) {
        sum += alpha * input.powf(rho);
    }
    total_factor_productivity * sum.powf(1.0 / rho)
}

pub fn marginal_utility_cobb_douglas(inputs: &[f64], alphas: &[f64], total_factor_productivity: f64, index: usize) -> f64 {
    if inputs[index] == 0.0 {
        return f64::INFINITY;
    }
    let u = cobb_douglas(inputs, alphas, total_factor_productivity);
    u * alphas[index] / inputs[index]
}

pub fn optimize_cobb_douglas_fixed_prices(alphas: &[f64], prices: &[f64], budget: f64) -> Vec<f64> {
    let mut bundle = vec![0.0; alphas.len()];
    for i in 0..alphas.len() {
        if prices[i] > 0.0 {
            bundle[i] = alphas[i] * budget / prices[i];
        }
    }
    bundle
}
