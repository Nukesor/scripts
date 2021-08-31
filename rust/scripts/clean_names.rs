use std::env::current_dir;

use script_utils::*;

fn main() -> Result<()> {
    setup();

    let current_dir = current_dir()?;
    let dirs = read_dir_or_fail(current_dir, Some(FileType::Directory))?;

    for dir in dirs {
        let path = dir.path();
        let filename = path.file_name().ok_or(anyhow!(format!(
            "Couldn't get filename from path: {:?}",
            path
        )))?;
        let filename = filename.to_str().ok_or(anyhow!(format!(
            "Filename contains invalid utf8: {:?}",
            filename
        )))?;

        let mut chars: Vec<char> = filename.clone().chars().collect();
        // Check for each brace, if there is are matching pairs of braces in the path.
        // Everything between those braces will be removed.
        for (start, end) in get_braces() {
            // Search for pairs, until we no longer find some.
            loop {
                let mut start_index: Option<usize> = None;
                let mut end_index: Option<usize> = None;
                for (index, c) in chars.iter().enumerate() {
                    if start_index.is_none() && *c == start {
                        start_index = Some(index);
                    }

                    // We found an matching end brace.
                    // Break the loop, remove the matching part of the name and start anew.
                    if start_index.is_some() && *c == end {
                        end_index = Some(index);
                        break;
                    }
                }

                // We couldn't find a matching pair. This is our exit condition.
                if start_index.is_none() || end_index.is_none() {
                    break;
                }

                // Remove the subslice.
                chars.drain(start_index.unwrap()..end_index.unwrap());
            }
        }

        // Get all indices of invalid characters.
        let mut chars_to_remove = Vec::new();
        let invalid_characters = invalid_characters();
        for (index, c) in chars.iter().enumerate() {
            if invalid_characters.contains(c) {
                chars_to_remove.push(index);
            }
        }

        // Remove all invalid char from the back to the front.
        // Needed to prevent invalid indices due to inded shifting on removal.
        chars_to_remove.reverse();
        for c in chars_to_remove {
            chars.remove(c);
        }

        // Compile the modified character list into a new string.
        let mut new_name: String = chars.into_iter().collect();

        // Remove trailing/preceeding whitespaces
        for c in trailing_chars() {
            while let Some(stripped) = new_name.strip_prefix(c) {
                new_name = stripped.to_owned();
            }
            while let Some(stripped) = new_name.strip_suffix(c) {
                new_name = stripped.to_owned();
            }
        }

        let mut new_path = path.clone();
        new_path.set_file_name(&new_name);

        println!(
            "Moving a) to b):\na) '{:?}'\nb) '{:?}'\n",
            filename, new_name
        );
        std::fs::rename(path, new_path)?;
    }

    Ok(())
}

fn get_braces() -> Vec<(char, char)> {
    vec![('[', ']'), ('(', ')'), ('{', '}')]
}

/// Return all chars that are considered invalid in our filename.
fn invalid_characters() -> Vec<char> {
    let mut chars = vec![';'];
    for (start, end) in get_braces() {
        chars.push(start);
        chars.push(end);
    }

    chars
}

fn trailing_chars() -> Vec<char> {
    vec![' ', '\n', '\r']
}
