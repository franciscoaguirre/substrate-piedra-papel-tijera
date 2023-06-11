use crate::{
	mock::*,
	tipos::{CantidadDeJugadores, Etapa, Jugada},
	utils::conseguir_compromiso,
	Error, Event,
};
use frame_support::{assert_noop, assert_ok};

#[test]
fn registrar_funciona() {
	new_test_ext().execute_with(|| {
		// Comenzamos en la etapa correcta
		assert_eq!(
			PiedraPapelTijera::etapa(),
			Etapa::EsperandoJugadores(CantidadDeJugadores::Cero)
		);

		assert_ok!(PiedraPapelTijera::registrar(Origin::signed(1)));
		let jugadores = PiedraPapelTijera::jugadores();
		assert_eq!(jugadores.len(), 1);
		assert_eq!(jugadores.first().unwrap(), &(1, None, None));

		assert_eq!(PiedraPapelTijera::etapa(), Etapa::EsperandoJugadores(CantidadDeJugadores::Uno));

		// Un mismo usuario no puede registrarse dos veces.
		assert_noop!(PiedraPapelTijera::registrar(Origin::signed(1)), Error::<Test>::YaRegistrado);

		assert_ok!(PiedraPapelTijera::registrar(Origin::signed(2)));
		let jugadores = PiedraPapelTijera::jugadores(); // Recargar vector
		assert_eq!(jugadores.len(), 2);
		assert_eq!(jugadores.first().unwrap(), &(1, None, None));
		assert_eq!(jugadores.last().unwrap(), &(2, None, None));

		// Solo pueden haber dos jugadores. Cuando están todas el juego pasa de etapa.
		assert_noop!(
			PiedraPapelTijera::registrar(Origin::signed(3)),
			Error::<Test>::EtapaIncorrecta
		);
	});
}

#[test]
fn commit_y_reveal_funcionan() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		// Empezamos con los dos jugadores.
		assert_ok!(PiedraPapelTijera::registrar(Origin::signed(1)));
		assert_ok!(PiedraPapelTijera::registrar(Origin::signed(2)));

		let jugada_1 = Jugada::Papel;
		let (hash_1, nonce_1) = conseguir_compromiso::<Test>(jugada_1);

		assert_noop!(
			PiedraPapelTijera::commit(Origin::signed(3), hash_1),
			Error::<Test>::NoEsJugador
		);
		assert_ok!(PiedraPapelTijera::commit(Origin::signed(1), hash_1));

		assert_eq!(PiedraPapelTijera::etapa(), Etapa::Commit(CantidadDeJugadores::Uno));

		let jugada_2 = Jugada::Piedra;
		let (hash_2, nonce_2) = conseguir_compromiso::<Test>(jugada_2);
		assert_ok!(PiedraPapelTijera::commit(Origin::signed(2), hash_2));

		let jugadores = PiedraPapelTijera::jugadores();
		assert_eq!(jugadores.first().unwrap(), &(1, Some(hash_1), None));
		assert_eq!(jugadores.last().unwrap(), &(2, Some(hash_2), None));

		assert_eq!(PiedraPapelTijera::etapa(), Etapa::Reveal(CantidadDeJugadores::Cero));

		assert_noop!(
			PiedraPapelTijera::reveal(Origin::signed(3), Jugada::Papel, 0),
			Error::<Test>::NoEsJugador
		);

		assert_ok!(PiedraPapelTijera::reveal(Origin::signed(1), jugada_1, nonce_1));
		assert_noop!(
			PiedraPapelTijera::reveal(Origin::signed(2), jugada_1, nonce_1),
			Error::<Test>::HashIncorrecto
		);
		assert_ok!(PiedraPapelTijera::reveal(Origin::signed(2), jugada_2, nonce_2));
		assert_eq!(PiedraPapelTijera::etapa(), Etapa::Fin);

		assert_ok!(PiedraPapelTijera::finalizar_juego(Origin::signed(1)));
		System::assert_last_event(Event::Fin { ganador: Some(1) }.into());
	});
}
