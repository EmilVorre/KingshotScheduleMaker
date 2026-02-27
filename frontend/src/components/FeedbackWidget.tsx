import { useState } from 'react'
import { api } from '../api/client'

export type FeedbackType = 'bug' | 'feature' | 'general'

const TYPES: { value: FeedbackType; label: string; icon: string }[] = [
  { value: 'bug', label: 'Bug', icon: 'fa-bug' },
  { value: 'feature', label: 'Feature', icon: 'fa-star' },
  { value: 'general', label: 'General', icon: 'fa-comment' },
]

const STORAGE_KEY = 'feedback-widget-hide-until'
const HIDE_DAYS = 1

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

export default function FeedbackWidget() {
  const [open, setOpen] = useState(false)
  const [visible, setVisible] = useState(() => {
    const until = getHideUntil()
    return until == null || Date.now() > until
  })
  const [type, setType] = useState<FeedbackType>('general')
  const [text, setText] = useState('')
  const [sending, setSending] = useState(false)
  const [sent, setSent] = useState(false)
  const [error, setError] = useState<string | null>(null)

  function handleClose() {
    setOpen(false)
    setText('')
    setError(null)
    if (sent) setSent(false)
  }

  function handleDismiss() {
    const until = Date.now() + HIDE_DAYS * 24 * 60 * 60 * 1000
    setHideUntil(until)
    setVisible(false)
    setOpen(false)
  }

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault()
    if (!text.trim()) return
    setSending(true)
    setError(null)
    const { ok } = await api.submitFeedback({ type, text: text.trim() })
    setSending(false)
    if (ok) {
      setSent(true)
      setText('')
      setTimeout(() => {
        setOpen(false)
        setSent(false)
      }, 1500)
    } else {
      setError('Failed to send feedback')
    }
  }

  if (!visible && !open) return null

  return (
    <>
      {/* Floating button - bottom right, two-part like reference */}
      <div className="fixed bottom-24 right-6 z-50 flex items-center">
        {!open ? (
          <>
            <button
              onClick={() => setOpen(true)}
              className="flex items-center gap-2 px-4 py-2.5 bg-blue-600 hover:bg-blue-700 text-white rounded-l-lg rounded-r-none shadow-lg transition-all"
              title="Feedback"
            >
              <i className="fas fa-comment-dots"></i>
              <span>Feedback</span>
            </button>
            <button
              onClick={handleDismiss}
              className="flex items-center justify-center w-10 h-[42px] bg-blue-700 hover:bg-blue-800 text-gray-200 rounded-r-lg border-l border-blue-500/50 transition-colors"
              title="Dismiss"
            >
              <i className="fas fa-times"></i>
            </button>
          </>
        ) : null}
      </div>

      {/* Modal */}
      {open && (
        <div className="fixed inset-0 z-40 flex items-center justify-center p-4 bg-black/60" onClick={handleClose}>
          <div
            className="bg-gray-800 rounded-xl border border-gray-700 shadow-2xl w-full max-w-md"
            onClick={(e) => e.stopPropagation()}
          >
            <div className="flex items-center justify-between p-4 border-b border-gray-700">
              <h3 className="text-lg font-bold text-white">Send Feedback</h3>
              <button
                onClick={handleClose}
                className="p-2 text-gray-400 hover:text-white hover:bg-gray-700 rounded-lg transition-colors"
              >
                <i className="fas fa-times"></i>
              </button>
            </div>

            <form onSubmit={handleSubmit} className="p-4 space-y-4">
              <div>
                <label className="block text-sm font-medium text-gray-300 mb-2">Type</label>
                <div className="flex gap-2">
                  {TYPES.map((t) => (
                    <button
                      key={t.value}
                      type="button"
                      onClick={() => setType(t.value)}
                      className={`flex items-center gap-1.5 px-3 py-2 rounded-lg text-sm font-medium transition-all ${
                        type === t.value
                          ? 'bg-blue-600 text-white'
                          : 'bg-gray-700 text-gray-300 hover:bg-gray-600 border border-gray-600'
                      }`}
                    >
                      <i className={`fas ${t.icon} text-xs`}></i>
                      {t.label}
                    </button>
                  ))}
                </div>
              </div>

              <div>
                <label className="block text-sm font-medium text-gray-300 mb-2">Your feedback</label>
                <textarea
                  value={text}
                  onChange={(e) => setText(e.target.value)}
                  placeholder="Share your thoughts with us..."
                  rows={4}
                  className="w-full px-4 py-3 bg-gray-700 border border-gray-600 rounded-lg text-white placeholder-gray-500 focus:border-blue-500 focus:ring-2 focus:ring-blue-500/50 outline-none resize-y"
                  required
                />
              </div>

              {error && (
                <p className="text-red-400 text-sm">
                  <i className="fas fa-exclamation-circle mr-1"></i>
                  {error}
                </p>
              )}

              <button
                type="submit"
                disabled={sending || !text.trim()}
                className="w-full py-3 bg-blue-600 hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed text-white font-semibold rounded-lg transition-all"
              >
                {sending ? (
                  <i className="fas fa-spinner fa-spin mr-2"></i>
                ) : sent ? (
                  <i className="fas fa-check mr-2"></i>
                ) : null}
                {sending ? 'Sending...' : sent ? 'Sent!' : 'Send Feedback'}
              </button>
            </form>
          </div>
        </div>
      )}
    </>
  )
}
