#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/v3/runtime/frame>
pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

mod tipos;
use tipos::*;

#[cfg(feature = "std")]
mod utils;

use frame_support::{
	pallet_prelude::*,
	traits::{Currency, ExistenceRequirement::KeepAlive, Get},
	PalletId,
};
use frame_system::pallet_prelude::*;
use sp_runtime::traits::{AccountIdConversion, Hash, SaturatedConversion, Saturating};

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		type Currency: Currency<Self::AccountId>;

		#[pallet::constant]
		type PalletId: Get<PalletId>;

		#[pallet::constant]
		type TokensParaJugar: Get<BalanceDe<Self>>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::getter(fn jugadores)]
	pub type Jugadores<T> = StorageValue<_, BoundedVec<Jugador<T>, ConstU32<2>>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn etapa)]
	pub type EtapaDelJuego<T> = StorageValue<_, Etapa, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// El usuario se registró.
		Registrado { quien: CuentaDe<T> },
		/// El usuario comprometió su jugada.
		Comprometido { quien: CuentaDe<T>, hash: HashDe<T> },
		/// El usuario reveló su jugada.
		Revelado { quien: CuentaDe<T>, jugada: Jugada },
		/// El juego terminó, anunciar ganador, puede haber sido empate.
		Fin { ganador: Option<CuentaDe<T>> },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// El usuario ya se registró al juego, no puede volver a hacerlo.
		YaRegistrado,
		/// Usuario ya se comprometió a una jugada.
		YaComprometido,
		/// Usuario ya reveló su jugada.
		YaRevelado,
		/// Usuario trató de registrarse, comprometerse o revelar en la etapa incorrecta.
		EtapaIncorrecta,
		/// Usuario intentó comprometerse o revelar, pero no está jugando.
		NoEsJugador,
		/// Usuario intentó revelar una jugada pero el hash fue incorrecto.
		HashIncorrecto,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Registra al usuario para jugar
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn registrar(origen: OriginFor<T>) -> DispatchResult {
			// Revisar etapa del juego
			let mut etapa = EtapaDelJuego::<T>::get();
			ensure!(matches!(etapa, Etapa::EsperandoJugadores(_)), Error::<T>::EtapaIncorrecta);

			let quien = ensure_signed(origen)?;
			let mut jugadores = Jugadores::<T>::get();
			// Si la etapa es correcta, hay máximo un jugador en el arreglo.
			if let Some(primer_jugador) = jugadores.first() {
				ensure!(primer_jugador.0 != quien, Error::<T>::YaRegistrado);
			}

			let jugador = (quien.clone(), None, None); // Jugadores comienzan sin jugada ni compromiso.
			jugadores.force_push(jugador); // Sabemos que no está lleno el arreglo porque la etapa es correcta.
			Jugadores::<T>::set(jugadores);

			// Guardamos la apuesta de este jugador
			T::Currency::transfer(
				&quien,
				&Self::account_id(),
				T::TokensParaJugar::get(),
				KeepAlive,
			)?;

			// Avanzar etapa
			etapa.next();
			EtapaDelJuego::<T>::set(etapa);

			Self::deposit_event(Event::Registrado { quien });

			Ok(())
		}

		/// Jugador se compromete a una jugada, no la puede cambiar.
		#[pallet::weight(10_000 * T::DbWeight::get().writes(1))]
		pub fn commit(origen: OriginFor<T>, hash: HashDe<T>) -> DispatchResult {
			let mut etapa = EtapaDelJuego::<T>::get();
			ensure!(matches!(etapa, Etapa::Commit(_)), Error::<T>::EtapaIncorrecta);

			let quien = ensure_signed(origen)?;
			let mut jugadores = Jugadores::<T>::get();
			let mut encontrado = false;
			for jugador in jugadores.iter_mut() {
				if jugador.0 == quien {
					// Asegurarnos que el jugador no cambie su jugada.
					ensure!(jugador.1 == None, Error::<T>::YaComprometido);
					jugador.1 = Some(hash);
					encontrado = true;
				}
			}
			ensure!(encontrado, Error::<T>::NoEsJugador);
			Jugadores::<T>::set(jugadores);

			// Avanzar etapa
			etapa.next();
			EtapaDelJuego::<T>::set(etapa);

			Self::deposit_event(Event::Comprometido { quien, hash });

			Ok(())
		}

		/// Jugador revela su jugada.
		/// Al elegir usar `u128` como el tipo del nonce, hay 3,4x10^38 posibles nonces.
		#[pallet::weight(10_000 * T::DbWeight::get().writes(1))]
		pub fn reveal(origen: OriginFor<T>, jugada: Jugada, nonce: u128) -> DispatchResult {
			let mut etapa = EtapaDelJuego::<T>::get();
			ensure!(matches!(etapa, Etapa::Reveal(_)), Error::<T>::EtapaIncorrecta);

			let quien = ensure_signed(origen)?;
			let mut jugadores = Jugadores::<T>::get();
			let mut encontrado = false;
			for jugador in jugadores.iter_mut() {
				if jugador.0 == quien {
					// Asegurarnos que el jugador no haya revelado antes.
					ensure!(jugador.2 == None, Error::<T>::YaRevelado);
					let concatenacion = jugada.using_encoded(|slice_1| {
						nonce.using_encoded(|slice_2| [slice_1, slice_2].concat())
					});
					let hash = <T as frame_system::Config>::Hashing::hash_of(&concatenacion);
					ensure!(
						hash == jugador.1.expect("Debe haber un hash en esta etapa"),
						Error::<T>::HashIncorrecto
					);
					jugador.2 = Some(jugada);
					encontrado = true;
				}
			}
			ensure!(encontrado, Error::<T>::NoEsJugador);
			Jugadores::<T>::set(jugadores);

			// Avanzar etapa
			etapa.next();
			EtapaDelJuego::<T>::set(etapa);

			Self::deposit_event(Event::Revelado { quien, jugada });

			Ok(())
		}

		/// Terminar el juego y declarar el ganador.
		/// Solo puede llamarse si ambos jugadores revelaron sus jugadas.
		#[pallet::weight(10_000 * T::DbWeight::get().writes(1))]
		pub fn finalizar_juego(_origen: OriginFor<T>) -> DispatchResult {
			let etapa = EtapaDelJuego::<T>::get();
			ensure!(etapa == Etapa::Fin, Error::<T>::EtapaIncorrecta);

			let jugadores = Jugadores::<T>::get();
			let jugador_1 = jugadores.first().expect("En esta etapa existen los dos jugadores");
			let jugada_1 = jugador_1.2.expect("En esta etapa existen las jugadas");
			let jugador_2 = jugadores.last().expect("En esta etapa existen los dos jugadores");
			let jugada_2 = jugador_2.2.expect("En esta etapa existen las jugadas");

			// Lógica para decidir el ganador
			use Jugada::*;
			let ganador = match (jugada_1, jugada_2) {
				(Papel, Piedra) | (Piedra, Tijera) | (Tijera, Papel) => Some(jugador_1.0.clone()),
				(Piedra, Papel) | (Tijera, Piedra) | (Papel, Tijera) => Some(jugador_2.0.clone()),
				_ => None, // Empate
			};

			// Pagarle al ganador
			if let Some(ganador) = ganador.clone() {
				let (cuenta_pallet, total) = Self::pot();
				let result = T::Currency::transfer(&cuenta_pallet, &ganador, total, KeepAlive);
				debug_assert!(result.is_ok()); // No manejamos error
			} else {
				// Empate, devolver las cantidades originales
				let (cuenta_pallet, total) = Self::pot();
				let total: u128 = total.saturated_into();
				let cantidad_original = total.saturating_div(2);
				let result = T::Currency::transfer(
					&cuenta_pallet,
					&jugador_1.0,
					cantidad_original.saturated_into(),
					KeepAlive,
				);
				debug_assert!(result.is_ok());
				let result = T::Currency::transfer(
					&cuenta_pallet,
					&jugador_2.0,
					cantidad_original.saturated_into(),
					KeepAlive,
				);
				debug_assert!(result.is_ok());
			}

			Self::deposit_event(Event::Fin { ganador });

			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {
	pub fn account_id() -> T::AccountId {
		T::PalletId::get().into_account_truncating()
	}

	fn pot() -> (T::AccountId, BalanceDe<T>) {
		let account_id = Self::account_id();
		let balance = T::Currency::free_balance(&account_id)
			// Restamos el mínimo para no borrar la cuenta
			.saturating_sub(T::Currency::minimum_balance());

		(account_id, balance)
	}
}
