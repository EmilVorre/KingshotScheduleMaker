import { useEffect, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { useAuth } from '../context/AuthContext'
import { api, type AllianceApplication } from '../api/client'

export default function AdminAllianceApplicationsPage() {
  const { isAdmin, isValid, refresh } = useAuth()
  const navigate = useNavigate()
  const [applications, setApplications] = useState<AllianceApplication[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [actioning, setActioning] = useState<string | null>(null)

  useEffect(() => {
    if (isValid !== null && !isAdmin) {
      navigate('/', { replace: true })
    }
  }, [isAdmin, isValid, navigate])

  async function loadApplications() {
    setLoading(true)
    setError(null)
    const { ok, data } = await api.listAllianceApplications()
    setLoading(false)
    if (ok && data?.applications) {
      setApplications(data.applications)
    } else {
      setError('Failed to load applications')
    }
  }

  useEffect(() => {
    if (isAdmin) loadApplications()
  }, [isAdmin])

  async function approve(id: string) {
    setActioning(id)
    setError(null)
    const { ok } = await api.approveAllianceApplication(id)
    setActioning(null)
    if (ok) {
      await refresh()
      loadApplications()
    } else {
      setError('Failed to approve')
    }
  }

  async function reject(id: string) {
    setActioning(id)
    setError(null)
    const { ok } = await api.rejectAllianceApplication(id)
    setActioning(null)
    if (ok) {
      loadApplications()
    } else {
      setError('Failed to reject')
    }
  }

  const pending = applications.filter((a) => a.status === 'pending')
  const resolved = applications.filter((a) => a.status !== 'pending')

  return (
    <div className="container mx-auto px-4 py-8 max-w-4xl">
      <div className="text-center mb-8">
        <div className="inline-block bg-indigo-900/50 rounded-full p-4 mb-4">
          <i className="fas fa-file-signature text-indigo-400 text-3xl"></i>
        </div>
        <h1 className="text-3xl font-bold text-white mb-2">Alliance Applications</h1>
        <p className="text-gray-400">Review and approve or reject alliance access applications</p>
      </div>

      {error && (
        <div className="mb-4 p-4 bg-red-900/50 border-l-4 border-red-500 text-red-200 rounded-lg">
          <i className="fas fa-exclamation-circle mr-2"></i>
          {error}
        </div>
      )}

      {loading && (
        <div className="text-center py-12">
          <i className="fas fa-spinner fa-spin text-4xl text-indigo-400 mb-4"></i>
          <p className="text-gray-400">Loading applications...</p>
        </div>
      )}

      {!loading && (
        <div className="space-y-8">
          {pending.length > 0 && (
            <div className="bg-gray-800 rounded-xl shadow-xl p-6 border border-gray-700">
              <h2 className="text-lg font-semibold text-white mb-4">
                <i className="fas fa-clock mr-2"></i>Pending ({pending.length})
              </h2>
              <div className="space-y-4">
                {pending.map((app) => (
                  <div
                    key={app.id}
                    className="p-4 bg-gray-700/50 rounded-lg border border-gray-600"
                  >
                    <div className="flex flex-wrap justify-between items-start gap-4">
                      <div>
                        <p className="font-medium text-white">
                          [{app.alliance_tag}] {app.alliance_name}
                        </p>
                        <p className="text-sm text-gray-400 mt-1">
                          Account: {app.account_name} · Server {app.server_number}
                        </p>
                        <p className="text-sm text-gray-400">
                          Contact: {app.contact_player_id} · {new Date(app.submitted_at).toLocaleString()}
                        </p>
                      </div>
                      <div className="flex gap-2">
                        <button
                          onClick={() => approve(app.id)}
                          disabled={!!actioning}
                          className="px-4 py-2 bg-green-600 hover:bg-green-500 disabled:opacity-50 text-white rounded-lg text-sm font-medium"
                        >
                          {actioning === app.id ? (
                            <i className="fas fa-spinner fa-spin"></i>
                          ) : (
                            <>
                              <i className="fas fa-check mr-1"></i>Approve
                            </>
                          )}
                        </button>
                        <button
                          onClick={() => reject(app.id)}
                          disabled={!!actioning}
                          className="px-4 py-2 bg-red-600 hover:bg-red-500 disabled:opacity-50 text-white rounded-lg text-sm font-medium"
                        >
                          {actioning === app.id ? (
                            <i className="fas fa-spinner fa-spin"></i>
                          ) : (
                            <>
                              <i className="fas fa-times mr-1"></i>Reject
                            </>
                          )}
                        </button>
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}

          {resolved.length > 0 && (
            <div className="bg-gray-800 rounded-xl shadow-xl p-6 border border-gray-700">
              <h2 className="text-lg font-semibold text-white mb-4">
                <i className="fas fa-history mr-2"></i>Resolved ({resolved.length})
              </h2>
              <div className="space-y-3">
                {resolved.map((app) => (
                  <div
                    key={app.id}
                    className="p-3 bg-gray-700/30 rounded-lg border border-gray-600/50 flex flex-wrap justify-between items-center gap-2"
                  >
                    <div>
                      <span className="font-medium text-white">
                        [{app.alliance_tag}] {app.alliance_name}
                      </span>
                      <span className="text-gray-400 text-sm ml-2">
                        {app.account_name} · {app.status}
                      </span>
                    </div>
                    <span className="text-xs text-gray-500">
                      {new Date(app.submitted_at).toLocaleString()}
                    </span>
                  </div>
                ))}
              </div>
            </div>
          )}

          {applications.length === 0 && !loading && (
            <div className="bg-gray-800 rounded-xl shadow-xl p-12 text-center border border-gray-700">
              <i className="fas fa-inbox text-4xl text-gray-500 mb-4"></i>
              <p className="text-gray-400">No alliance applications yet</p>
            </div>
          )}
        </div>
      )}
    </div>
  )
}
