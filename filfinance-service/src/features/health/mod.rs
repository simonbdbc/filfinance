//! Feature technique dediee au healthcheck public.

// Le transport HTTP reste isole du bootstrap applicatif.
pub(crate) mod http;

#[cfg(test)]
mod tests;
