//! Tests du lecteur de logs admin.
//!
//! Ces tests fixent le contrat d'exploitation: retention J/J-1, pagination filtree,
//! refus des jours hors retention et lecture incremental du tail.

// `std::fs` ecrit de vrais fichiers temporaires pour tester le meme chemin que la prod.
use std::{
    fs,
    // `PathBuf` transporte un chemin possede, pratique pour construire un dossier par test.
    path::PathBuf,
};

// `OffsetDateTime` donne la date UTC courante utilisee par le service de logs.
use time::OffsetDateTime;

use crate::{
    // La config reduit la retention et la taille des pages pour garder les tests lisibles.
    config::LoggingConfig,
    // `test_api` expose les fonctions bloquantes sans passer par le WebSocket admin.
    features::logs::service::test_api::{
        AdminLogReadParams, AdminLogTailParams, format_day, list_log_days_blocking,
        read_log_page_blocking, read_tail_append_blocking, read_tail_snapshot_blocking,
    },
};

fn temp_log_dir() -> PathBuf {
    // OS-reserved unique directory: parallel tests must never share log files.
    tempfile::Builder::new()
        .prefix("filfinance-log-tests-")
        .tempdir()
        .expect("isolated test log dir")
        .keep()
}

fn logging_config(directory: PathBuf) -> LoggingConfig {
    // Cette config active les fichiers, limite la retention a J/J-1 et force des petites pages.
    LoggingConfig {
        file_enabled: true,
        // `to_string_lossy` suffit ici car le chemin temporaire est controle par le test.
        directory: directory.to_string_lossy().into_owned(),
        retention_days: 2,
        admin_max_lines: 2,
    }
}

#[test]
fn admin_log_days_lists_only_available_retained_days() {
    // Scenario: l'admin ne doit voir que les fichiers dans la fenetre de retention.
    let directory = temp_log_dir();
    fs::create_dir_all(&directory).expect("test log dir should be created");
    // Les helpers du service comparent les noms de fichier au jour UTC courant.
    let today = OffsetDateTime::now_utc().date();
    let yesterday = today.previous_day().unwrap_or(today);
    // Ce troisieme jour sort de la retention de deux jours configuree par le test.
    let stale_day = yesterday.previous_day().unwrap_or(yesterday);
    // On cree explicitement les trois fichiers pour verifier le filtrage, pas l'absence disque.
    fs::write(
        directory.join(format!("{}.log", format_day(today))),
        "INFO today\n",
    )
    .expect("today log should be written");
    fs::write(
        directory.join(format!("{}.log", format_day(yesterday))),
        "WARN yesterday\n",
    )
    .expect("yesterday log should be written");
    fs::write(
        directory.join(format!("{}.log", format_day(stale_day))),
        "ERROR stale\n",
    )
    .expect("stale log should be written");

    // Le service doit lister les jours disponibles sans exposer le fichier stale.
    let config = logging_config(directory.clone());
    let days = list_log_days_blocking(&config).expect("log days should load");

    // L'ordre attendu reste du plus recent au plus ancien, contrat utile au viewer admin.
    assert_eq!(
        days.days
            .iter()
            .map(|day| day.day.clone())
            .collect::<Vec<_>>(),
        vec![format_day(today), format_day(yesterday)]
    );

    // Nettoyage best-effort: l'echec de suppression ne doit pas masquer le resultat du test.
    let _ = fs::remove_dir_all(directory);
}

#[test]
fn admin_log_page_paginates_and_filters_lines() {
    // Scenario: une page admin peut combiner filtre niveau, filtre texte et pagination.
    let directory = temp_log_dir();
    fs::create_dir_all(&directory).expect("test log dir should be created");
    let today = OffsetDateTime::now_utc().date();
    // Le fichier contient plusieurs niveaux pour prouver que seul WARN+cloudflare ressort.
    fs::write(
        directory.join(format!("{}.log", format_day(today))),
        "INFO startup ok\nWARN cloudflare challenge\nERROR tts failed\nINFO shutdown\n",
    )
    .expect("today log should be written");

    let config = logging_config(directory.clone());
    // `limit=1` verrouille le contrat de pagination meme si un seul item matche.
    let first_page = read_log_page_blocking(
        &config,
        AdminLogReadParams {
            day: format_day(today),
            limit: Some(1),
            cursor: None,
            level: Some("warn".to_owned()),
            query: Some("cloudflare".to_owned()),
        },
    )
    .expect("first log page should load");

    // La ligne WARN est a l'index curseur 1 car les curseurs suivent la position du fichier.
    assert_eq!(first_page.items.len(), 1);
    assert_eq!(first_page.items[0].cursor, 1);
    assert_eq!(first_page.items[0].level.as_deref(), Some("warn"));
    // Aucun autre item ne matche les deux filtres; `has_more` doit donc rester faux.
    assert!(!first_page.has_more);

    let _ = fs::remove_dir_all(directory);
}

#[test]
fn admin_log_page_rejects_days_outside_retention() {
    // Scenario: meme si un client forge une date, le service refuse les jours hors J/J-1.
    let directory = temp_log_dir();
    let today = OffsetDateTime::now_utc().date();
    let yesterday = today.previous_day().unwrap_or(today);
    let stale_day = yesterday.previous_day().unwrap_or(yesterday);
    let config = logging_config(directory);

    // Aucun fichier n'est necessaire: le rejet porte sur la date demandee avant la lecture disque.
    let error = read_log_page_blocking(
        &config,
        AdminLogReadParams {
            day: format_day(stale_day),
            limit: None,
            cursor: None,
            level: None,
            query: None,
        },
    )
    .expect_err("stale log day must be rejected");

    // Le message reste explicite pour le runbook admin et pour le debug de l'interface.
    assert!(
        error
            .to_string()
            .contains("outside the retained J/J-1 window")
    );
}

#[test]
fn admin_log_tail_snapshot_keeps_latest_matching_lines() {
    // Scenario: un snapshot tail doit renvoyer les dernieres lignes qui matchent les filtres.
    let directory = temp_log_dir();
    fs::create_dir_all(&directory).expect("test log dir should be created");
    let today = OffsetDateTime::now_utc().date();
    // Deux WARN encadrent une ERROR; le filtre doit garder les curseurs reels 1 et 3.
    fs::write(
        directory.join(format!("{}.log", format_day(today))),
        "INFO first\nWARN second\nERROR third\nWARN fourth\n",
    )
    .expect("today log should be written");

    let config = logging_config(directory.clone());
    // Le snapshot retourne aussi un curseur reutilisable par la lecture append.
    let (snapshot, cursor) = read_tail_snapshot_blocking(
        &config,
        AdminLogTailParams {
            day: Some(format_day(today)),
            limit: Some(2),
            level: Some("warn".to_owned()),
            query: None,
        },
    )
    .expect("tail snapshot should load");

    // Les deux WARN sont conserves, avec les curseurs originaux du fichier.
    assert_eq!(snapshot.items.len(), 2);
    assert_eq!(snapshot.items[0].cursor, 1);
    assert_eq!(snapshot.items[1].cursor, 3);
    // `next_cursor=4` signifie que la prochaine lecture append commence apres la ligne 3.
    assert_eq!(snapshot.next_cursor, 4);
    assert_eq!(cursor.next_cursor, 4);
    // La taille disque est exposee pour que le client detecte rotations ou truncations.
    assert!(snapshot.file_size_bytes > 0);

    let _ = fs::remove_dir_all(directory);
}

#[test]
fn admin_log_tail_append_reads_only_new_lines() {
    // Scenario: apres un snapshot, la lecture append ne renvoie que les lignes ajoutees.
    let directory = temp_log_dir();
    fs::create_dir_all(&directory).expect("test log dir should be created");
    let today = OffsetDateTime::now_utc().date();
    let path = directory.join(format!("{}.log", format_day(today)));
    // Etat initial du fichier avant que le viewer admin ne commence a suivre le tail.
    fs::write(&path, "INFO first\n").expect("today log should be written");

    let config = logging_config(directory.clone());
    // On capture le curseur apres la premiere ligne pour simuler une connexion deja ouverte.
    let (_, cursor) = read_tail_snapshot_blocking(
        &config,
        AdminLogTailParams {
            day: Some(format_day(today)),
            limit: Some(2),
            level: None,
            query: None,
        },
    )
    .expect("tail snapshot should load");

    // Le fichier est reecrit avec une deuxieme ligne; le service compare via le curseur.
    fs::write(&path, "INFO first\nERROR second\n").expect("today log should be appended");
    let (append, next_cursor) = read_tail_append_blocking(cursor).expect("tail append should load");

    // Seule la nouvelle ligne ERROR doit sortir, pas la ligne INFO deja vue par le snapshot.
    assert_eq!(append.items.len(), 1);
    assert_eq!(append.items[0].cursor, 1);
    assert_eq!(append.items[0].level.as_deref(), Some("error"));
    // Le nouveau curseur avance a 2 pour que le prochain append reparte apres ERROR.
    assert_eq!(append.next_cursor, 2);
    assert_eq!(next_cursor.next_cursor, 2);

    let _ = fs::remove_dir_all(directory);
}

#[cfg(unix)]
#[test]
fn admin_logs_refuse_external_symlink_and_replacement() {
    use std::os::unix::fs::symlink;
    let sandbox = tempfile::tempdir().unwrap();
    let directory = sandbox.path().join("logs");
    fs::create_dir(&directory).unwrap();
    let today = format_day(OffsetDateTime::now_utc().date());
    let path = directory.join(format!("{today}.log"));
    let outside = sandbox.path().join("outside.txt");
    fs::write(&outside, "SYNTHETIC_OUTSIDE_CANARY\n").unwrap();
    let config = logging_config(directory);
    let params = || AdminLogTailParams {
        day: Some(today.clone()),
        limit: None,
        level: None,
        query: None,
    };
    // Absent files remain a valid empty tail; a later malicious replacement fails.
    let (empty, cursor) = read_tail_snapshot_blocking(&config, params()).unwrap();
    assert!(empty.items.is_empty());
    symlink(&outside, &path).unwrap();
    assert!(
        read_log_page_blocking(
            &config,
            AdminLogReadParams {
                day: today.clone(),
                limit: None,
                cursor: None,
                level: None,
                query: None
            }
        )
        .is_err()
    );
    assert!(read_tail_snapshot_blocking(&config, params()).is_err());
    assert!(read_tail_append_blocking(cursor).is_err());
    assert!(list_log_days_blocking(&config).unwrap().days.is_empty());
}

#[cfg(unix)]
#[test]
fn admin_logs_refuse_fifo_without_opening_or_waiting() {
    use std::{ffi::CString, os::unix::ffi::OsStrExt};
    let directory = tempfile::tempdir().unwrap();
    let today = format_day(OffsetDateTime::now_utc().date());
    let path = directory.path().join(format!("{today}.log"));
    let name = CString::new(path.as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
    let config = logging_config(directory.path().to_path_buf());
    assert!(
        read_log_page_blocking(
            &config,
            AdminLogReadParams {
                day: today.clone(),
                limit: None,
                cursor: None,
                level: None,
                query: None
            }
        )
        .is_err()
    );
    assert!(
        read_tail_snapshot_blocking(
            &config,
            AdminLogTailParams {
                day: Some(today),
                limit: None,
                level: None,
                query: None
            }
        )
        .is_err()
    );
    assert!(list_log_days_blocking(&config).unwrap().days.is_empty());
}

#[test]
fn admin_log_concurrent_snapshot_and_append_do_not_replay_consumed_lines() {
    use std::{
        io::Write,
        sync::{Arc, Barrier},
    };
    let directory = tempfile::tempdir().unwrap();
    let today = format_day(OffsetDateTime::now_utc().date());
    let path = directory.path().join(format!("{today}.log"));
    fs::write(&path, "INFO initial\n").unwrap();
    let barrier = Arc::new(Barrier::new(2));
    let writer_barrier = Arc::clone(&barrier);
    let writer = std::thread::spawn(move || {
        let mut file = fs::OpenOptions::new().append(true).open(path).unwrap();
        writer_barrier.wait();
        for index in 0..2000 {
            file.write_all(format!("INFO marker-{index}\n").as_bytes())
                .unwrap();
        }
    });
    let mut config = logging_config(directory.path().to_path_buf());
    config.admin_max_lines = 3000;
    barrier.wait();
    let (snapshot, cursor) = read_tail_snapshot_blocking(
        &config,
        AdminLogTailParams {
            day: Some(today),
            limit: Some(3000),
            level: None,
            query: None,
        },
    )
    .unwrap();
    writer.join().unwrap();
    let (append, cursor) = read_tail_append_blocking(cursor).unwrap();
    let texts: Vec<_> = snapshot
        .items
        .iter()
        .chain(append.items.iter())
        .map(|line| line.text.as_str())
        .collect();
    assert_eq!(texts.len(), 2001);
    for index in 0..2000 {
        let expected = format!("INFO marker-{index}");
        assert_eq!(texts.iter().filter(|text| **text == expected).count(), 1);
    }
    assert!(
        read_tail_append_blocking(cursor)
            .unwrap()
            .0
            .items
            .is_empty()
    );
}
