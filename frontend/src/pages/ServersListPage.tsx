import { useEffect, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { api } from '../api/client'

interface Server {
  account_name: string
  server_number: number
}

export default function ServersListPage() {
  const navigate = useNavigate()
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [servers, setServers] = useState<Server[]>([])

  useEffect(() => {
    api.getServers().then(({ ok, data, error: err }) => {
      if (ok && data?.servers) setServers(data.servers)
      else setError(err || 'Failed to load servers')
      setLoading(false)
    })
  }, [])

  function viewSchedule(accountName: string, serverNumber: number) {
    navigate(`/view/${accountName}/${serverNumber}`)
  }

  return (
    <div className="container mx-auto px-4 py-8 max-w-4xl">
      <header className="text-center mb-12">
        <h1 className="text-4xl font-bold text-blue-400 mb-4">
          <i className="fas fa-search mr-3"></i>Find Schedule
        </h1>
        <p className="text-gray-400">Select a server to view its schedule</p>
      </header>

      <main>
        {loading && (
          <div className="bg-gray-800 rounded-lg shadow-xl p-12 text-center border border-gray-700">
            <i className="fas fa-spinner fa-spin text-4xl text-blue-400 mb-4"></i>
            <p className="text-xl text-gray-400">Loading servers...</p>
          </div>
        )}
        {error && (
          <div className="bg-red-900/50 border-l-4 border-red-500 text-red-200 p-4 rounded-lg">
            <i className="fas fa-exclamation-circle mr-2"></i>{error}
          </div>
        )}
        {!loading && !error && servers.length === 0 && (
          <div className="bg-gray-800 rounded-lg shadow-xl p-12 text-center border border-gray-700">
            <i className="fas fa-inbox text-4xl text-gray-500 mb-4"></i>
            <p className="text-xl text-gray-400">No servers found</p>
            <p className="text-gray-500 mt-2">Create an account to get started</p>
          </div>
        )}
        {!loading && !error && servers.length > 0 && (
          <div className="space-y-3">
            {servers.map((server) => (
              <button
                key={`${server.account_name}-${server.server_number}`}
                onClick={() => viewSchedule(server.account_name, server.server_number)}
                className="w-full bg-gray-800 rounded-lg shadow-lg p-6 border border-gray-700 hover:border-blue-500 hover:shadow-xl transition-all cursor-pointer text-left"
              >
                <div className="flex items-center justify-between">
                  <div>
                    <h3 className="text-2xl font-bold text-white mb-1">{server.account_name}</h3>
                    <p className="text-gray-400">
                      <i className="fas fa-server mr-2"></i>Server {server.server_number}
                    </p>
                  </div>
                  <i className="fas fa-chevron-right text-gray-400 text-xl"></i>
                </div>
              </button>
            ))}
          </div>
        )}
      </main>

    </div>
  )
}
