export function hapticLight() {
  navigator?.vibrate?.(10)
}

export function hapticMedium() {
  navigator?.vibrate?.(20)
}
