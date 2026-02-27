import { useEffect, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { useAuth } from '../context/AuthContext'
import { api } from '../api/client'

interface AccountRow {
  account_name: string
  server_number: number
  in_game_name: string
  admin: boolean
}

export default function AdminResourcesPage() {
  const { isAdmin, isValid } = useAuth()
  const navigate = useNavigate()
  const [accounts, setAccounts] = useState<AccountRow[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [updating, setUpdating] = useState<string | null>(null)

  useEffect(() => {
    if (isValid !== null && !isAdmin) {
      navigate('/', { replace: true })
    }
  }, [isAdmin, isValid, navigate])

  async function loadAccounts() {
    setLoading(true)
    setError(null)
    const { ok, data } = await api.listAdminAccounts()
    setLoading(false)
    if (ok && data?.accounts) {
      setAccounts(data.accounts)
    } else {
      setError((data as { error?: string })?.error || 'Failed to load accounts')
    }
  }

  useEffect(() => {
    loadAccounts()
  }, [])

  async function toggleAdmin(accountName: string, currentAdmin: boolean) {
    setUpdating(accountName)
    setError(null)
    const { ok, data } = await api.setAdmin(accountName, !currentAdmin)
    setUpdating(null)
    if (ok && data?.success) {
      setAccounts((prev) =>
        prev.map((a) =>
          a.account_name === accountName ? { ...a, admin: !currentAdmin } : a
        )
      )
    } else {
      setError((data as { error?: string })?.error || 'Failed to update')
    }
  }

  return (
    <div className="container mx-auto px-4 py-8 max-w-4xl">
      <div className="text-center mb-8">
        <div className="inline-block bg-amber-900/50 rounded-full p-4 mb-4">
          <i className="fas fa-shield-alt text-amber-400 text-3xl"></i>
        </div>
        <h1 className="text-3xl font-bold text-white mb-2">Admin Resources</h1>
        <p className="text-gray-400">Manage admin privileges for accounts</p>
      </div>

      <div className="bg-gray-800 rounded-xl shadow-xl p-6 border border-gray-700">
        <h2 className="text-lg font-semibold text-white mb-4">
          <i className="fas fa-users-cog mr-2"></i>Manage Admins
        </h2>
        <p className="text-sm text-gray-400 mb-4">
          Grant or revoke admin access. Admins can access this panel and manage other admins.
        </p>

        {loading && (
          <div className="text-center py-12">
            <i className="fas fa-spinner fa-spin text-4xl text-blue-400 mb-4"></i>
            <p className="text-gray-400">Loading accounts...</p>
          </div>
        )}

        {error && (
          <div className="mb-4 p-4 bg-red-900/50 border-l-4 border-red-500 text-red-200 rounded-lg">
            <i className="fas fa-exclamation-circle mr-2"></i>
            {error}
          </div>
        )}

        {!loading && accounts.length === 0 && !error && (
          <p className="text-gray-400 text-center py-8">No accounts found.</p>
        )}

        {!loading && accounts.length > 0 && (
          <div className="overflow-x-auto">
            <table className="w-full text-left">
              <thead>
                <tr className="border-b border-gray-600">
                  <th className="py-3 px-4 text-sm font-medium text-gray-400 uppercase">Account</th>
                  <th className="py-3 px-4 text-sm font-medium text-gray-400 uppercase">Server</th>
                  <th className="py-3 px-4 text-sm font-medium text-gray-400 uppercase">In-game name</th>
                  <th className="py-3 px-4 text-sm font-medium text-gray-400 uppercase">Admin</th>
                  <th className="py-3 px-4 text-sm font-medium text-gray-400 uppercase">Actions</th>
                </tr>
              </thead>
              <tbody>
                {accounts.map((acc) => (
                  <tr key={acc.account_name} className="border-b border-gray-700/50 hover:bg-gray-700/30">
                    <td className="py-3 px-4 text-white font-medium">{acc.account_name}</td>
                    <td className="py-3 px-4 text-gray-300">{acc.server_number}</td>
                    <td className="py-3 px-4 text-gray-300">{acc.in_game_name || '-'}</td>
                    <td className="py-3 px-4">
                      {acc.admin ? (
                        <span className="text-amber-400">
                          <i className="fas fa-check-circle mr-1"></i>Yes
                        </span>
                      ) : (
                        <span className="text-gray-500">No</span>
                      )}
                    </td>
                    <td className="py-3 px-4">
                      <button
                        onClick={() => toggleAdmin(acc.account_name, acc.admin)}
                        disabled={!!updating}
                        className={`px-3 py-1.5 rounded-lg text-sm font-medium transition-all disabled:opacity-50 ${
                          acc.admin
                            ? 'bg-red-600/80 hover:bg-red-600 text-white'
                            : 'bg-green-600/80 hover:bg-green-600 text-white'
                        }`}
                      >
                        {updating === acc.account_name ? (
                          <i className="fas fa-spinner fa-spin"></i>
                        ) : acc.admin ? (
                          'Revoke admin'
                        ) : (
                          'Grant admin'
                        )}
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </div>
  )
}
