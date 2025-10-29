import { setTimeout } from 'timers/promises'

export default async function SlowPage() {
  // Must be higher than test timeout
  await setTimeout(10_000)
}
