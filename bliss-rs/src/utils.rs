pub fn mean<T: Clone + Into<f32>>(input: &[T]) -> f32 {
    input.iter().map(|x| x.clone().into() as f32).sum::<f32>() / input.len() as f32
}

pub fn number_crossings(input: &[f32]) -> usize {
    let j: usize;
    let mut ncr: usize = 0;

    for j in 1..input.len() {
        if input[j - 1] < 0. {
            if input[j] >= 0. {
                ncr += 1;
            }
        } else {
            if input[j] < 0. {
                ncr += 1;
            }
        }
    }
    ncr
}
