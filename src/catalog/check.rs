//! What the repositories release is hashed in a `SHA256SUMS` list (one per release, next to the assets; see
//! [`super::Repo::sha_url`]), and the downloads of [`super`] verify against it: a mismatch or an entry the list does not
//! hold refuses the download, and a repository with no list yet only warns. Verifying keeps a replaced or tampered
//! download from being unpacked as a plugin, theme or mapping the user installs.

use std::path::Path;

/// The SHA-256 of `file` as lowercase hex.
pub fn sha256(file: &Path) -> Result<String, String> {
    use sha2::Digest;
    let mut h = sha2::Sha256::new();
    let mut f = std::fs::File::open(file).map_err(|e| format!("{}: {e}", file.display()))?;
    std::io::copy(&mut f, &mut h).map_err(|e| format!("{}: {e}", file.display()))?;
    Ok(format!("{:x}", h.finalize()))
}

/// `text` is a `SHA256SUMS` (`<hash> <name>` per line): the entry `name` must hold the hash of `file`.
pub fn verify(text: &str, name: &str, file: &Path) -> Result<(), String> {
    let got = sha256(file)?;
    let line = text
        .lines()
        .find(|l| l.split_once(' ').is_some_and(|(_, n)| n.trim_start() == name));
    let Some(line) = line else {
        return Err(format!(
            "{name}: its SHA256SUMS does not have an entry for this file, the download is refused"
        ));
    };
    let want = line.trim_start().split(' ').next().unwrap_or("");
    if want.len() != 64 || !want.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(format!("the SHA256SUMS entry {name:?} does not look like one"));
    }
    if want.eq_ignore_ascii_case(&got) {
        Ok(())
    } else {
        Err(format!(
            "{name}: the download hashes {got}, its SHA256SUMS says {want}: the file is refused"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(tag: &str, bytes: &[u8]) -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!("ok-check-{tag}-{}", std::process::id()));
        std::fs::write(&p, bytes).unwrap();
        p
    }

    #[test]
    fn sha256_is_the_one_every_answersheet_has() {
        let (zero, abc) = (temp("zero", b""), temp("abc", b"abc"));
        assert_eq!(
            sha256(&zero).unwrap(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256(&abc).unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        std::fs::remove_file(zero).ok();
        std::fs::remove_file(abc).ok();
    }

    #[test]
    fn the_list_entries_match_a_matching_file() {
        let f = temp("e", b"abc");
        let list = format!("68b329da9893e34099c7d8ad5cb9c940  readme.txt\nba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad  abc.zip\n");
        assert_eq!(verify(&list, "abc.zip", &f), Ok(()));
        assert!(verify(&list, "readme.txt", &f).is_err(), "another entry hashes it");
        assert!(
            verify(&list, "missing.zip", &f)
                .unwrap_err()
                .contains("does not have an entry")
        );
        // a name with spaces holds (the two spaces follow the hash), and a wrong hash says what is what
        std::fs::remove_file(&f).ok();
        let f = temp("e2", b"x");
        let list = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855  my plugin zip\n";
        assert!(
            verify(list, "my plugin zip", &f)
                .unwrap_err()
                .contains("the download hashes")
        );
        assert!(
            verify("zz  my plugin zip\n", "my plugin zip", &f)
                .unwrap_err()
                .contains("does not look like one")
        );
        std::fs::remove_file(f).ok();
    }
}
