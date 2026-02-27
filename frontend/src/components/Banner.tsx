import { useState, useEffect } from 'react'

const STORAGE_KEY = 'banner-hide-until'
const HIDE_DAYS = 14

function getHideUntil(): number | null {
  try {
    const raw = localStorage.getItem(STORAGE_KEY)
    if (!raw) return null
    const ts = parseInt(raw, 10)
    return isNaN(ts) ? null : ts
  } catch {
    return null
  }
}

function setHideUntil(ts: number) {
  try {
    localStorage.setItem(STORAGE_KEY, String(ts))
  } catch {
    // ignore
  }
}

export default function Banner() {
  const [visible, setVisible] = useState(false)

  useEffect(() => {
    const until = getHideUntil()
    const now = Date.now()
    setVisible(until == null || now > until)
  }, [])

  function handleDismiss() {
    const until = Date.now() + HIDE_DAYS * 24 * 60 * 60 * 1000
    setHideUntil(until)
    setVisible(false)
  }

  if (!visible) return null

  return (
    <div className="flex-shrink-0 flex items-center justify-between gap-4 px-4 py-2.5 bg-amber-900/60 border-b border-amber-700/50 text-amber-100 text-sm">
      <p className="flex items-center gap-2">
        <i className="fas fa-info-circle text-amber-400"></i>
        This site is still in progress. Please give us feedback.
      </p>
      <button
        onClick={handleDismiss}
        className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-amber-800/60 hover:bg-amber-800 text-amber-200 hover:text-white transition-colors text-xs font-medium"
      >
        Hide for 14 days
      </button>
    </div>
  )
}
