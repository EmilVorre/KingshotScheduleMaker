export function parseTimeToMinutes(timeStr: string): number {
  const parts = timeStr.split(':')
  if (parts.length !== 2) return 0
  const hours = parseInt(parts[0], 10) || 0
  const minutes = parseInt(parts[1], 10) || 0
  return hours * 60 + minutes
}

export function sortTimeSlots(
  timeSlotMap: Record<string, { requests: number }>,
  startTime: string
): Record<string, { requests: number }> {
  const entries = Object.entries(timeSlotMap)
  const startMinutes = parseTimeToMinutes(startTime)
  entries.sort((a, b) => {
    const minutesA = parseTimeToMinutes(a[0])
    const minutesB = parseTimeToMinutes(b[0])
    const adjustedA = minutesA < startMinutes ? minutesA + 24 * 60 : minutesA
    const adjustedB = minutesB < startMinutes ? minutesB + 24 * 60 : minutesB
    return adjustedA - adjustedB
  })
  return Object.fromEntries(entries)
}

export function getSubmissionValue(submission: Record<string, unknown>, header: string): string {
  if (!submission || typeof submission !== 'object') return ''
  const keys = Object.keys(submission)
  const findKey = (patterns: string[]) => {
    for (const pattern of patterns) {
      const normalizedPattern = pattern.toLowerCase().replace(/\s+/g, ' ').trim()
      const found = keys.find((k) => {
        const normalizedKey = k
          .toLowerCase()
          .replace(/\s+/g, ' ')
          .replace(/\n/g, ' ')
          .trim()
        return normalizedKey.includes(normalizedPattern)
      })
      if (found) return found
    }
    return null
  }
  const columnMap: Record<string, string | null> = {
    Timestamp: 'timestamp',
    Name: findKey(['character name']),
    Alliance: findKey(['alliance do you belong']),
    'Construction speedups': findKey([
      'speedups do you plan to use on construction day',
      'construction day',
      'speedups',
      'construction',
    ]),
    Truegold: findKey(['how much truegold do you plan', 'truegold', 'plan to', 'spend']),
    'want Construction?': findKey(['do you want a construction day appointment']),
    'Construction times': findKey([
      'times are you available for your construction day appointment',
      'construction day appointment',
      'utc time',
      'construction',
    ]),
    'Research Speedups': findKey([
      'speedups do you plan to use on research day',
      'research day',
      'speedups',
      'research',
    ]),
    'Truegold Dust': findKey(['how much truegold dust do you plan', 'truegold dust']),
    'want Research?': findKey(['do you want a research day appointment']),
    'Research times': findKey([
      'times are you available for your research day appointment',
      'research day appointment',
      'utc time',
      'research',
    ]),
    'Troop Speedups': findKey([
      'speedups do you plan to use on troops training day',
      'troops training day',
      'speedups',
      'troops',
    ]),
    'Want troops?': findKey(['do you want a troops training day appointment']),
    'Troop times': findKey([
      'times are you available for your troops training day appointment',
      'troops training day appointment',
      'utc time',
      'troops',
    ]),
  }
  const columnKey = columnMap[header]
  if (columnKey && submission[columnKey] !== undefined) {
    return String(submission[columnKey])
  }
  return String(submission[header] ?? '')
}

export type BuildingResearchDaySlot = 'monday' | 'tuesday' | 'friday_full' | 'friday_sat'

export function daySlotToTimes(
  slot: BuildingResearchDaySlot
): { start_time: string; end_time?: string } {
  switch (slot) {
    case 'friday_sat':
      return { start_time: '10:00', end_time: undefined }
    case 'monday':
    case 'tuesday':
    case 'friday_full':
    default:
      return { start_time: '00:00', end_time: undefined }
  }
}

export function getTotal(data: {
  construction_requests: number
  research_requests: number
  troops_requests: number
}) {
  return data.construction_requests + data.research_requests + data.troops_requests
}

export function formatDate(dateString?: string) {
  if (!dateString) return 'Unknown'
  try {
    return new Date(dateString).toLocaleString()
  } catch {
    return dateString
  }
}

export function copyToClipboard(text: string, inputId?: string) {
  const fullUrl = text.startsWith('/') ? `${window.location.origin}${text}` : text
  if (navigator.clipboard?.writeText) {
    navigator.clipboard.writeText(fullUrl)
  } else if (inputId) {
    const input = document.getElementById(inputId) as HTMLInputElement
    if (input) {
      input.value = fullUrl
      input.select()
      document.execCommand('copy')
    }
  }
}
