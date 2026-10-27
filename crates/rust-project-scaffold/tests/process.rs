#![cfg(unix)]

use rust_project_scaffold::{CancellationToken, Error, Programs};
use std::{fs, os::unix::fs::PermissionsExt, path::Path, time::Duration};

fn directory() -> tempfile::TempDir {
    tempfile::tempdir_in(concat!(env!("CARGO_MANIFEST_DIR"), "/.tmp")).unwrap()
}

#[tokio::test]
async fn subprocess_retains_arguments_working_directory_and_failure_details() {
    let directory = directory();
    let programs = Programs::default();
    let output = programs
        .run(
            Path::new("/bin/sh"),
            &[
                "-c",
                "printf '%s\\n' \"$PWD\" \"$1\"",
                "fixture",
                "spaces and $literal",
            ],
            Some(directory.path()),
            &CancellationToken::new(),
        )
        .await
        .unwrap();
    assert_eq!(
        output,
        format!("{}\nspaces and $literal\n", directory.path().display()).as_bytes()
    );
    let error = programs
        .run(
            Path::new("/bin/sh"),
            &["-c", "printf failure >&2; exit 7"],
            None,
            &CancellationToken::new(),
        )
        .await
        .unwrap_err();
    assert!(matches!(error, Error::Process { .. }));
    assert!(error.to_string().contains("failure"));
    assert!(error.to_string().contains('7'));
    assert!(
        programs
            .run(
                &directory.path().join("missing"),
                &[],
                None,
                &CancellationToken::new()
            )
            .await
            .is_err()
    );
}

#[tokio::test]
async fn timeout_and_cancellation_kill_and_reap_the_owned_process_group() {
    for cancel in [false, true] {
        let directory = directory();
        let script = directory.path().join("child.sh");
        fs::write(&script, "#!/bin/sh\nprintf '%s' \"$$\" > parent.pid\nsleep 60 &\nprintf '%s' \"$!\" > descendant.pid\nwait\n").unwrap();
        fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
        let programs = Programs {
            timeout: Duration::from_secs(2),
            ..Programs::default()
        };
        let token = CancellationToken::new();
        let cancellation = token.clone();
        let ready = directory.path().join("descendant.pid");
        let cancellation_task = tokio::spawn(async move {
            if cancel {
                tokio::time::timeout(Duration::from_secs(1), async {
                    while !ready.exists() {
                        tokio::time::sleep(Duration::from_millis(2)).await;
                    }
                })
                .await
                .unwrap();
                cancellation.cancel();
            }
        });
        let error = programs
            .run(&script, &[], Some(directory.path()), &token)
            .await
            .unwrap_err();
        cancellation_task.await.unwrap();
        assert!(if cancel {
            matches!(error, Error::Cancelled)
        } else {
            error.to_string().contains("timed out")
        });
        let parent: i32 = fs::read_to_string(directory.path().join("parent.pid"))
            .unwrap()
            .parse()
            .unwrap();
        // wait() has consumed the actual child's status; it is no longer live or a zombie.
        assert_eq!(unsafe { libc::kill(parent, 0) }, -1);
        // A reparented descendant may briefly remain as a zombie until the host reaps it.
        #[cfg(target_os = "linux")]
        {
            let descendant: i32 = fs::read_to_string(directory.path().join("descendant.pid"))
                .unwrap()
                .parse()
                .unwrap();
            let state = fs::read_to_string(format!("/proc/{descendant}/stat"));
            assert!(state.is_err() || state.unwrap().split_once(") ").unwrap().1.starts_with('Z'));
        }
    }
}
