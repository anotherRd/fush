pub fn db_array_placeholders(data_length: usize) -> String {
    std::iter::repeat("?")
        .take(data_length)
        .collect::<Vec<_>>()
        .join(",")
}