/** Replicate backend calculate_time_slots: Slot 1 = start, Slot 2 = start+15min, Slot 3+ = prev+30min. Returns slot numbers (1,2,3...) for API. */
export function calculateTimeSlots(startTime: string, endTime?: string | null): Array<{ value: number; label: string }> {
  const parseMinutes = (t: string): number => {
    const parts = t.split(':')
    if (parts.length !== 2) return 0
    return (parseInt(parts[0], 10) || 0) * 60 + (parseInt(parts[1], 10) || 0)
  }
  const toTime = (m: number): string => {
    const mm = ((m % (24 * 60)) + 24 * 60) % (24 * 60)
    const h = Math.floor(mm / 60)
    const min = mm % 60
    return `${String(h).padStart(2, '0')}:${String(min).padStart(2, '0')}`
  }

  const startMinutes = parseMinutes(startTime)
  let endMinutes = endTime ? parseMinutes(endTime) : startMinutes + 24 * 60
  if (endMinutes < startMinutes) endMinutes += 24 * 60

  const slots: Array<{ value: number; label: string }> = []
  let currentMinutes = startMinutes
  let slotNum = 1

  slots.push({ value: slotNum, label: toTime(currentMinutes) })
  slotNum++
  currentMinutes += 15

  if (currentMinutes < endMinutes) {
    slots.push({ value: slotNum, label: toTime(currentMinutes % (24 * 60)) })
    slotNum++
    while (slotNum <= 200) {
      currentMinutes += 30
      const cm = currentMinutes % (24 * 60)
      if (currentMinutes >= endMinutes) break
      slots.push({ value: slotNum, label: toTime(cm) })
      slotNum++
    }
  }

  return slots
}
