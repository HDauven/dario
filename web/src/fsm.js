// JS mirror of dario_fsm/src/lib.rs — keep in sync with the Rust transition table.

export const STATE = {
  Regular: 0,
  Super: 1,
  Fire: 2,
  Cape: 3,
  GameOver: 4,
};

export const EVENT = {
  Espresso: 0,
  ChiliPepper: 1,
  TableClothCape: 2,
  TakeDamage: 3,
  Revive: 4,
};

const { Regular, Super, Fire, Cape, GameOver } = STATE;
const { Espresso, ChiliPepper, TableClothCape, TakeDamage, Revive } = EVENT;

export function transition(state, event) {
  if (state === Regular && event === Espresso) return Super;
  if (state === Regular && event === ChiliPepper) return Fire;
  if (state === Regular && event === TableClothCape) return Cape;
  if (state === Super && event === ChiliPepper) return Fire;
  if (state === Super && event === TableClothCape) return Cape;
  if (state === Fire && event === TableClothCape) return Cape;
  if (state === Cape && event === ChiliPepper) return Fire;
  if (state === Regular && event === TakeDamage) return GameOver;
  if (
    (state === Super || state === Fire || state === Cape) &&
    event === TakeDamage
  ) {
    return Regular;
  }
  if (state === GameOver && event === Revive) return Regular;
  return state;
}
