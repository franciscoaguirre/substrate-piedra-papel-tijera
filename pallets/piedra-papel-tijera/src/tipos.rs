use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::traits::Currency;
use scale_info::TypeInfo;

#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone, TypeInfo, MaxEncodedLen)]
pub enum CantidadDeJugadores {
	Cero,
	Uno,
}

#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone, TypeInfo, MaxEncodedLen)]
pub enum Etapa {
	EsperandoJugadores(CantidadDeJugadores),
	Commit(CantidadDeJugadores),
	Reveal(CantidadDeJugadores),
	Fin,
}

impl Default for Etapa {
	fn default() -> Self {
		Self::EsperandoJugadores(CantidadDeJugadores::Cero)
	}
}

impl Etapa {
	/// Avanza hacia la siguiente etapa del juego.
	/// Si la etapa ya es Etapa::Fin, es idempotente.
	pub fn next(&mut self) {
		use CantidadDeJugadores::*;
		use Etapa::*;

		*self = match *self {
			EsperandoJugadores(Cero) => EsperandoJugadores(Uno),
			EsperandoJugadores(Uno) => Commit(Cero),
			Commit(Cero) => Commit(Uno),
			Commit(Uno) => Reveal(Cero),
			Reveal(Cero) => Reveal(Uno),
			Reveal(Uno) => Fin,
			Fin => Fin,
		};
	}
}

#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone, Copy, TypeInfo, MaxEncodedLen)]
pub enum Jugada {
	Piedra,
	Papel,
	Tijera,
}

pub type CuentaDe<T> = <T as frame_system::Config>::AccountId;
pub type HashDe<T> = <T as frame_system::Config>::Hash;
pub type BalanceDe<T> = <<T as crate::Config>::Currency as Currency<CuentaDe<T>>>::Balance;

pub type Jugador<T> = (CuentaDe<T>, Option<HashDe<T>>, Option<Jugada>);
