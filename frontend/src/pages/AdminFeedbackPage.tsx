import { useEffect, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { useAuth } from '../context/AuthContext'
import { api } from '../api/client'

interface FeedbackItem {
  id: string
  type: string
  text: string
  created_at: string
}

const TYPE_CONFIG: Record<string, { label: string; icon: string; bg: string; border: string }> = {
  bug: { label: 'Bug', icon: 'fa-bug', bg: 'bg-red-900/40', border: 'border-red-600/50' },
  feature: { label: 'Feature', icon: 'fa-star', bg: 'bg-amber-900/40', border: 'border-amber-600/50' },
  general: { label: 'General', icon: 'fa-comment', bg: 'bg-blue-900/40', border: 'border-blue-600/50' },
}

function formatDate(iso: string) {
  try {
    const d = new Date(iso)
    return d.toLocaleDateString(undefined, {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    })
  } catch {
    return iso
  }
}

export default function AdminFeedbackPage() {
  const { isAdmin, isValid } = useAuth()
  const navigate = useNavigate()
  const [feedback, setFeedback] = useState<FeedbackItem[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [archiving, setArchiving] = useState<string | null>(null)

  useEffect(() => {
    if (isValid !== null && !isAdmin) {
      navigate('/', { replace: true })
    }
  }, [isAdmin, isValid, navigate])

  async function loadFeedback() {
    setLoading(true)
    setError(null)
    const { ok, data } = await api.listFeedback()
    setLoading(false)
    if (ok && data?.feedback) {
      setFeedback(data.feedback)
    } else {
      setError((data as { error?: string })?.error || 'Failed to load feedback')
    }
  }

  useEffect(() => {
    loadFeedback()
  }, [])

  async function handleArchive(id: string) {
    setArchiving(id)
    setError(null)
    const { ok } = await api.archiveFeedback(id)
    setArchiving(null)
    if (ok) {
      setFeedback((prev) => prev.filter((f) => f.id !== id))
    } else {
      setError('Failed to archive feedback')
    }
  }

  return (
    <div className="container mx-auto px-4 py-8 max-w-4xl">
      <div className="text-center mb-8">
        <div className="inline-block bg-amber-900/50 rounded-full p-4 mb-4">
          <i className="fas fa-comments text-amber-400 text-3xl"></i>
        </div>
        <h1 className="text-3xl font-bold text-white mb-2">Feedback</h1>
        <p className="text-gray-400">User feedback and improvement suggestions</p>
      </div>

      <div className="bg-gray-800 rounded-xl shadow-xl p-6 border border-gray-700">
        <h2 className="text-lg font-semibold text-white mb-4">
          <i className="fas fa-inbox mr-2"></i>All Feedback
        </h2>

        {loading && (
          <div className="text-center py-12">
            <i className="fas fa-spinner fa-spin text-4xl text-blue-400 mb-4"></i>
            <p className="text-gray-400">Loading feedback...</p>
          </div>
        )}

        {error && (
          <div className="mb-4 p-4 bg-red-900/50 border-l-4 border-red-500 text-red-200 rounded-lg">
            <i className="fas fa-exclamation-circle mr-2"></i>
            {error}
          </div>
        )}

        {!loading && feedback.length === 0 && !error && (
          <p className="text-gray-400 text-center py-8">No feedback yet.</p>
        )}

        {!loading && feedback.length > 0 && (
          <div className="space-y-4">
            {feedback.map((item) => {
              const config = TYPE_CONFIG[item.type] ?? TYPE_CONFIG.general
              return (
                <div
                  key={item.id}
                  className={`rounded-lg border p-4 ${config.bg} ${config.border}`}
                >
                  <div className="flex items-start justify-between gap-2 mb-2">
                    <div className="flex items-center gap-2 flex-wrap">
                      <span className={`inline-flex items-center gap-1.5 px-2 py-0.5 rounded text-sm font-medium ${
                        item.type === 'bug' ? 'bg-red-600/60 text-red-100' :
                        item.type === 'feature' ? 'bg-amber-600/60 text-amber-100' :
                        'bg-blue-600/60 text-blue-100'
                      }`}>
                        <i className={`fas ${config.icon} text-xs`}></i>
                        {config.label}
                      </span>
                      <span className="text-xs text-gray-500">
                        {formatDate(item.created_at)}
                      </span>
                    </div>
                    <button
                      onClick={() => handleArchive(item.id)}
                      disabled={!!archiving}
                      className="flex-shrink-0 px-3 py-1.5 text-xs font-medium text-gray-400 hover:text-white hover:bg-gray-600/50 rounded-lg transition-colors disabled:opacity-50"
                      title="Archive (hide from list, keep saved)"
                    >
                      {archiving === item.id ? (
                        <i className="fas fa-spinner fa-spin"></i>
                      ) : (
                        <>
                          <i className="fas fa-archive mr-1"></i>
                          Archive
                        </>
                      )}
                    </button>
                  </div>
                  <p className="text-gray-200 whitespace-pre-wrap">{item.text}</p>
                </div>
              )
            })}
          </div>
        )}
      </div>
    </div>
  )
}
