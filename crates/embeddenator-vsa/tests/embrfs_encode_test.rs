use embeddenator_vsa::ReversibleVSAEncoder;

#[test]
fn test_encode_decode_per_chunk() {
    let mut encoder = ReversibleVSAEncoder::new();

    // Test with the same data as the test
    let data = b"This is a test of chunked encoding for longer data.";
    let chunk_size = 8;

    // Test using encode/decode on each chunk (like EmbrFS does)
    let mut total_correct = 0;
    let mut total_bytes = 0;

    for chunk in data.chunks(chunk_size) {
        let encoded = encoder.encode(chunk);
        let decoded = encoder.decode(&encoded, chunk.len());

        let correct = chunk
            .iter()
            .zip(decoded.iter())
            .filter(|(a, b)| a == b)
            .count();
        total_correct += correct;
        total_bytes += chunk.len();

        println!(
            "Chunk {:?} -> {:?} ({}/{} correct)",
            String::from_utf8_lossy(chunk),
            String::from_utf8_lossy(&decoded),
            correct,
            chunk.len()
        );
    }

    let accuracy = total_correct as f64 / total_bytes as f64;
    println!("\nOverall accuracy: {:.2}%", accuracy * 100.0);

    assert!(
        accuracy >= 0.8,
        "Accuracy {:.1}% is too low",
        accuracy * 100.0
    );
}
