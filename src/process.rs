use anyhow::Result;
use procfs::process::all_processes;

/// Get all cmdlines of currently running processes.
pub fn get_process_cmdlines(current_user_id: u32) -> Result<Vec<String>> {
    let processes = all_processes()?
        .filter_map(|process| process.ok())
        // We're only interested in alive processes that belong to the current user.
        .filter(|process| {
            let uid = if let Ok(uid) = process.uid() {
                uid
            } else {
                return false;
            };
            process.is_alive() && uid == current_user_id
        })
        .filter_map(|process| {
            // Don't include the process if we cannot get the cmdline.
            if let Ok(cmdline) = process.cmdline() {
                // Only get the first few strings which should include the name of the game.
                if cmdline.len() < 6 {
                    Some(cmdline.join(" "))
                } else {
                    let (left, _) = cmdline.split_at(5);
                    Some(left.join(" "))
                }
            } else {
                None
            }
        })
        .collect();

    Ok(processes)
}
