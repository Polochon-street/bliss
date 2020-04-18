pub fn mean<T: Clone + Into<f32>>(input: &[T]) -> f32 {
    input.iter().map(|x| x.clone().into() as f32).sum::<f32>() / input.len() as f32
}

// Essentia algorithm
// https://github.com/MTG/essentia/blob/master/src/algorithms/temporal/zerocrossingrate.cpp
pub fn number_crossings(input: &[f32]) -> u32 {
    let mut crossings = 0;
    let mut val = input[0];

    if val.abs() < 0. {
        val = 0.
    };
    let mut was_positive = val > 0.;
    let mut is_positive: bool;

    for sample in input {
        val = *sample;
        if val.abs() <= 0.0 {
            val = 0.0
        };
        is_positive = val > 0.;
        if was_positive != is_positive {
            crossings += 1;
            was_positive = is_positive;
        }
    }

    crossings
}

pub fn geometric_mean(input: &[f32]) -> f32 {
    let mut mean = 0.0;

    for &sample in input {
        if sample == 0.0 {
            return 0.0
        }
        mean += sample.ln();
    }

    mean /= input.len() as f32;

    mean.exp()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mean() {
        let numbers = vec![0.0, 1.0, 2.0, 3.0, 4.0];
        assert_eq!(2.0, mean(&numbers));
    }

    #[test]
    fn test_geometric_mean() {
        let numbers = vec![0.0, 1.0, 2.0, 3.0, 4.0];
        assert_eq!(0.0, geometric_mean(&numbers));

        let numbers = vec![4.0, 1.0, 0.03125];
        assert_eq!(0.5, geometric_mean(&numbers));
    }
}
