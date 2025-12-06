use std::path::PathBuf;

/// Chunk files into batches of max_size
pub fn chunk_files(files: &[PathBuf], max_size: usize) -> Vec<Vec<PathBuf>> {
    if max_size == 0 || files.len() <= max_size {
        return vec![files.to_vec()];
    }

    files.chunks(max_size).map(|c| c.to_vec()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_files_small() {
        let files: Vec<PathBuf> = (0..3)
            .map(|i| PathBuf::from(format!("file{}.py", i)))
            .collect();
        let chunks = chunk_files(&files, 10);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].len(), 3);
    }

    #[test]
    fn test_chunk_files_exact() {
        let files: Vec<PathBuf> = (0..10)
            .map(|i| PathBuf::from(format!("file{}.py", i)))
            .collect();
        let chunks = chunk_files(&files, 5);
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].len(), 5);
        assert_eq!(chunks[1].len(), 5);
    }

    #[test]
    fn test_chunk_files_remainder() {
        let files: Vec<PathBuf> = (0..7)
            .map(|i| PathBuf::from(format!("file{}.py", i)))
            .collect();
        let chunks = chunk_files(&files, 3);
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].len(), 3);
        assert_eq!(chunks[1].len(), 3);
        assert_eq!(chunks[2].len(), 1);
    }
}
