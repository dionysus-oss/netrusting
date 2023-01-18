#[cfg(test)]
pub fn dirty_to_bytes(repr: String) -> Vec<u8> {
    let mut result = Vec::with_capacity(repr.len());

    let labels = repr.split('.');

    for label in labels {
        result.push(label.len() as u8);
        for ch in label.chars() {
            result.push(ch as u8)
        }
    }

    result
}
