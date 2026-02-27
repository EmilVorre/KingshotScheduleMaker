import { useState } from 'react'
import { Link } from 'react-router-dom'
import { api } from '../api/client'

export default function CreateAccountPage() {
  const [creating, setCreating] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [accountCreated, setAccountCreated] = useState(false)
  const [scheduleUrl, setScheduleUrl] = useState('')
  const [dashboardUrl, setDashboardUrl] = useState('')
  const [form, setForm] = useState({
    account_name: '',
    server_number: null as number | null,
    password: '',
    in_game_name: '',
    player_id: '',
  })

  async function handleCreateAccount(e: React.FormEvent) {
    e.preventDefault()
    setCreating(true)
    setError(null)
    try {
      const { ok, data, error: err } = await api.createAccount({
        ...form,
        server_number: form.server_number ?? 1,
        player_id: form.player_id.trim() || undefined,
      })
      if (ok && data?.success) {
        setScheduleUrl(data.schedule_url || `/${form.account_name}/${form.server_number}`)
        setDashboardUrl(`/dashboard/${form.account_name}`)
        setAccountCreated(true)
      } else {
        setError((data as { message?: string })?.message || err || 'Failed to create account')
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Error')
    } finally {
      setCreating(false)
    }
  }

  if (accountCreated) {
    return (
      <div className="container mx-auto px-4 py-8 max-w-4xl">
        <header className="text-center mb-12">
          <h1 className="text-4xl font-bold text-blue-400 mb-4">
            <i className="fas fa-user-plus mr-3"></i>Create Account
          </h1>
          <p className="text-gray-400">Create a new account to manage your server schedule</p>
        </header>

        <div className="bg-gray-800 rounded-lg shadow-xl p-8 border border-gray-700">
          <div className="text-center">
            <div className="mb-6">
              <div className="inline-block bg-green-900/50 rounded-full p-4 mb-4">
                <i className="fas fa-check-circle text-green-400 text-4xl"></i>
              </div>
              <h2 className="text-2xl font-bold text-green-400 mb-2">Account Created Successfully!</h2>
              <p className="text-gray-400 mb-6">Your schedule URL is ready</p>
            </div>
            <div className="bg-gray-700 rounded-lg p-4 mb-6">
              <p className="text-sm text-gray-400 mb-2">Your Schedule URL:</p>
              <code className="text-blue-400 font-mono text-lg break-all">{scheduleUrl}</code>
            </div>
            <div className="flex gap-4 justify-center">
              <Link
                to={scheduleUrl}
                className="bg-blue-600 hover:bg-blue-700 text-white px-6 py-3 rounded-lg font-semibold transition-all shadow-lg hover:shadow-xl"
              >
                <i className="fas fa-calendar-check mr-2"></i>View Schedule
              </Link>
              <Link
                to={dashboardUrl}
                className="bg-green-600 hover:bg-green-700 text-white px-6 py-3 rounded-lg font-semibold transition-all shadow-lg hover:shadow-xl"
              >
                <i className="fas fa-sign-in-alt mr-2"></i>Login
              </Link>
            </div>
          </div>
        </div>
        <div className="text-center mt-8">
          <Link to="/" className="text-blue-400 hover:text-blue-300 transition-colors">
            <i className="fas fa-arrow-left mr-2"></i>Back to Home
          </Link>
        </div>
      </div>
    )
  }

  return (
    <div className="container mx-auto px-4 py-8 max-w-4xl">
      <header className="text-center mb-12">
        <h1 className="text-4xl font-bold text-blue-400 mb-4">
          <i className="fas fa-user-plus mr-3"></i>Create Account
        </h1>
        <p className="text-gray-400">Create a new account to manage your server schedule</p>
      </header>

      <div className="bg-gray-800 rounded-lg shadow-xl p-8 border border-gray-700">
        <div className="mb-8 space-y-3">
          <p className="text-sm font-medium text-gray-400">Or create with:</p>
          <div className="flex gap-3">
            <a
              href="/api/auth/discord"
              className="flex-1 flex items-center justify-center gap-2 px-4 py-3 bg-[#5865F2] hover:bg-[#4752C4] text-white rounded-lg font-medium transition-all"
            >
              <i className="fab fa-discord text-xl"></i>
              Discord
            </a>
            <a
              href="/api/auth/google"
              className="flex-1 flex items-center justify-center gap-2 px-4 py-3 bg-white hover:bg-gray-100 text-gray-800 rounded-lg font-medium transition-all border border-gray-300"
            >
              <i className="fab fa-google text-xl text-red-500"></i>
              Google
            </a>
          </div>
        </div>
        <form onSubmit={handleCreateAccount} className="space-y-6">
          <div>
            <label htmlFor="account_name" className="block text-sm font-semibold text-gray-300 mb-2">
              <i className="fas fa-user mr-2"></i>Account Name
            </label>
            <input
              type="text"
              id="account_name"
              value={form.account_name}
              onChange={(e) => setForm((f) => ({ ...f, account_name: e.target.value }))}
              required
              className="w-full px-4 py-3 bg-gray-700 border border-gray-600 rounded-lg text-white focus:border-blue-500 focus:ring-2 focus:ring-blue-500/50 outline-none transition-all"
              placeholder="Enter account name"
            />
            <p className="text-xs text-gray-500 mt-1">Used in your schedule URL</p>
          </div>
          <div>
            <label htmlFor="server_number" className="block text-sm font-semibold text-gray-300 mb-2">
              <i className="fas fa-server mr-2"></i>Server Number
            </label>
            <input
              type="number"
              id="server_number"
              value={form.server_number ?? ''}
              onChange={(e) => setForm((f) => ({ ...f, server_number: e.target.value ? parseInt(e.target.value, 10) : null }))}
              required
              min={1}
              className="w-full px-4 py-3 bg-gray-700 border border-gray-600 rounded-lg text-white focus:border-blue-500 focus:ring-2 focus:ring-blue-500/50 outline-none transition-all"
              placeholder="Enter server number"
            />
          </div>
          <div>
            <label htmlFor="in_game_name" className="block text-sm font-semibold text-gray-300 mb-2">
              <i className="fas fa-gamepad mr-2"></i>In-Game Name
            </label>
            <input
              type="text"
              id="in_game_name"
              value={form.in_game_name}
              onChange={(e) => setForm((f) => ({ ...f, in_game_name: e.target.value }))}
              required
              className="w-full px-4 py-3 bg-gray-700 border border-gray-600 rounded-lg text-white focus:border-blue-500 focus:ring-2 focus:ring-blue-500/50 outline-none transition-all"
              placeholder="Enter your in-game name"
            />
          </div>
          <div>
            <label htmlFor="player_id" className="block text-sm font-semibold text-gray-300 mb-2">
              <i className="fas fa-id-card mr-2"></i>Player ID
            </label>
            <input
              type="text"
              id="player_id"
              value={form.player_id}
              onChange={(e) => setForm((f) => ({ ...f, player_id: e.target.value }))}
              className="w-full px-4 py-3 bg-gray-700 border border-gray-600 rounded-lg text-white focus:border-blue-500 focus:ring-2 focus:ring-blue-500/50 outline-none transition-all"
              placeholder="Enter your Kingshot player ID (for profile picture)"
            />
            <p className="text-xs text-gray-500 mt-1">Optional. Used to display your profile picture from the game.</p>
          </div>
          <div>
            <label htmlFor="password" className="block text-sm font-semibold text-gray-300 mb-2">
              <i className="fas fa-lock mr-2"></i>Password
            </label>
            <input
              type="password"
              id="password"
              value={form.password}
              onChange={(e) => setForm((f) => ({ ...f, password: e.target.value }))}
              required
              className="w-full px-4 py-3 bg-gray-700 border border-gray-600 rounded-lg text-white focus:border-blue-500 focus:ring-2 focus:ring-blue-500/50 outline-none transition-all"
              placeholder="Enter password"
            />
            <p className="text-xs text-gray-500 mt-1">Password is used for authentication. Schedules are publicly viewable.</p>
          </div>
          <button
            type="submit"
            disabled={creating}
            className="w-full bg-blue-600 hover:bg-blue-700 text-white px-6 py-3 rounded-lg font-semibold transition-all shadow-lg hover:shadow-xl disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {creating ? (
              <>
                <i className="fas fa-spinner fa-spin mr-2"></i>Creating Account...
              </>
            ) : (
              <>
                <i className="fas fa-plus-circle mr-2"></i>Create Account
              </>
            )}
          </button>
        </form>
        {error && (
          <div className="mt-4 p-4 bg-red-900/50 border-l-4 border-red-500 text-red-200 rounded">
            <i className="fas fa-exclamation-circle mr-2"></i>{error}
          </div>
        )}
      </div>
      <div className="text-center mt-8">
        <Link to="/" className="text-blue-400 hover:text-blue-300 transition-colors">
          <i className="fas fa-arrow-left mr-2"></i>Back to Home
        </Link>
      </div>
    </div>
  )
}
