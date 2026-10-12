//! Facade de lecture HTTP du replay HLS du jour.

// Chaque sous-module porte une surface HTTP distincte du replay explicite.
mod coverage;
mod metadata;
mod playlist;
mod segment;
