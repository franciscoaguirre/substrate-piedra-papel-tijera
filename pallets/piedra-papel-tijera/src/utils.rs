use rand::Rng;

use crate::tipos::{HashDe, Jugada};
use codec::Encode;
use sp_runtime::traits::Hash;

/// Genera un nonce y devuelve un hash para `jugada`.
/// El nonce generado no es criptográficamente seguro.
/// Usa `T::Hashing` para generar el hash.
#[allow(dead_code)]
pub fn conseguir_compromiso<T: frame_system::Config>(jugada: Jugada) -> (HashDe<T>, u128) {
	let mut rng = rand::thread_rng();
	let nonce: u128 = rng.gen();
	let concatenacion =
		jugada.using_encoded(|slice_1| nonce.using_encoded(|slice_2| [slice_1, slice_2].concat()));
	let hash = T::Hashing::hash_of(&concatenacion);
	(hash, nonce)
}
