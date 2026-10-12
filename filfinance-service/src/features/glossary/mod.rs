//! Feature glossaire: corrections metier pour traduction et diction TTS.

// `domain` porte les DTO persistants et les helpers de validation.
pub(crate) mod domain;
// `http` expose le CRUD operateur du glossaire.
pub(crate) mod http;
// `pipeline` applique les termes actifs au flux EN -> FR -> vocal.
pub(crate) mod pipeline;
