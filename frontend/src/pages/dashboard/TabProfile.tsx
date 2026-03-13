import { Link } from 'react-router-dom'
import { api } from '../../api/client'

interface TabProfileProps {
  accountName: string | null
  serverNumber: number | null
  playerId: string | null
  inGameName: string | null
  friendCode: string | null
  profileEdit: { account_name: string; server_number: string; in_game_name: string }
  profileSaving: boolean
  profileError: string | null
  kingshotIdInput: string
  kingshotLookingUp: boolean
  kingshotError: string | null
  setProfileEdit: React.Dispatch<
    React.SetStateAction<{ account_name: string; server_number: string; in_game_name: string }>
  >
  setProfileSaving: (v: boolean) => void
  setProfileError: (v: string | null) => void
  setKingshotIdInput: (v: string) => void
  setKingshotLookingUp: (v: boolean) => void
  setKingshotError: (v: string | null) => void
  setServerNumber: (v: number | null) => void
  setInGameName: (v: string | null) => void
  setPlayerId: (v: string | null) => void
  refreshAuth: () => Promise<void>
  navigate: (to: string, opts?: { replace?: boolean }) => void
}

export default function TabProfile({
  accountName,
  serverNumber,
  playerId,
  inGameName,
  friendCode,
  profileEdit,
  profileSaving,
  profileError,
  kingshotIdInput,
  kingshotLookingUp,
  kingshotError,
  setProfileEdit,
  setProfileSaving,
  setProfileError,
  setKingshotIdInput,
  setKingshotLookingUp,
  setKingshotError,
  setServerNumber,
  setInGameName,
  setPlayerId,
  refreshAuth,
  navigate,
}: TabProfileProps) {
  return (
    <div className="max-w-2xl mx-auto">
      <div className="flex items-start gap-6 mb-8">
        <div className="w-20 h-20 rounded-full bg-purple-600 flex items-center justify-center text-white font-semibold text-2xl flex-shrink-0 overflow-hidden relative">
          <span className="absolute inset-0 flex items-center justify-center">
            {(inGameName || accountName) ? (inGameName || accountName)!.charAt(0).toUpperCase() : '?'}
          </span>
          {playerId && (
            <img
              key={playerId}
              src={`/api/avatar/${playerId}`}
              alt=""
              className="w-full h-full object-cover absolute inset-0 z-10"
              onError={(e) => {
                (e.target as HTMLImageElement).style.display = 'none'
              }}
            />
          )}
        </div>
        <div className="flex-1 min-w-0">
          <h2 className="text-2xl font-bold text-white mb-1">
            {inGameName || (accountName ? accountName.charAt(0).toUpperCase() + accountName.slice(1) : 'Account')}
          </h2>
          <p className="text-gray-400 text-sm mb-4">Schedule Maker account</p>
        </div>
      </div>

      <div className="space-y-4 mb-8">
        <div>
          <label className="block text-xs font-medium text-gray-500 uppercase tracking-wider mb-2">Account name</label>
          <input
            type="text"
            value={profileEdit.account_name}
            onChange={(e) => setProfileEdit((p) => ({ ...p, account_name: e.target.value }))}
            className="w-full px-4 py-3 bg-gray-700 border border-gray-600 rounded-lg text-white focus:border-blue-500 focus:ring-2 focus:ring-blue-500/50 outline-none"
            placeholder="Account name (used in URL)"
          />
        </div>
        <div>
          <label className="block text-xs font-medium text-gray-500 uppercase tracking-wider mb-2">Server</label>
          <input
            type="number"
            value={profileEdit.server_number}
            onChange={(e) => setProfileEdit((p) => ({ ...p, server_number: e.target.value }))}
            min={1}
            className="w-full px-4 py-3 bg-gray-700 border border-gray-600 rounded-lg text-white focus:border-blue-500 focus:ring-2 focus:ring-blue-500/50 outline-none"
            placeholder="Server number"
          />
        </div>
        <div>
          <label className="block text-xs font-medium text-gray-500 uppercase tracking-wider mb-2">In-game name</label>
          <input
            type="text"
            value={profileEdit.in_game_name}
            onChange={(e) => setProfileEdit((p) => ({ ...p, in_game_name: e.target.value }))}
            className="w-full px-4 py-3 bg-gray-700 border border-gray-600 rounded-lg text-white focus:border-blue-500 focus:ring-2 focus:ring-blue-500/50 outline-none"
            placeholder="Your Kingshot character name"
          />
        </div>
        <div>
          <label className="block text-xs font-medium text-gray-500 uppercase tracking-wider mb-2">Friend code</label>
          <div className="flex items-center gap-2">
            <code className="flex-1 px-4 py-3 bg-gray-700 border border-gray-600 rounded-lg text-indigo-300 font-mono">
              {friendCode || 'Loading...'}
            </code>
            <button
              type="button"
              onClick={() => friendCode && navigator.clipboard.writeText(friendCode)}
              className="px-4 py-3 bg-gray-600 hover:bg-gray-500 text-white rounded-lg"
              title="Copy"
            >
              <i className="fas fa-copy"></i>
            </button>
          </div>
          <p className="text-gray-500 text-xs mt-1">Share this so others can invite you to edit their alliance</p>
        </div>
        <button
          onClick={async () => {
            setProfileSaving(true)
            setProfileError(null)
            const { ok, data, error } = await api.updateProfile({
              account_name: profileEdit.account_name.trim() || undefined,
              server_number: profileEdit.server_number ? parseInt(profileEdit.server_number, 10) : undefined,
              in_game_name: profileEdit.in_game_name.trim() || undefined,
            })
            setProfileSaving(false)
            if (ok && data?.success) {
              setServerNumber(data.server_number ?? serverNumber)
              setInGameName(data.in_game_name ?? null)
              setProfileEdit((p) => ({
                ...p,
                account_name: data.account_name ?? p.account_name,
                server_number: String(data.server_number ?? p.server_number),
                in_game_name: data.in_game_name ?? p.in_game_name,
              }))
              await refreshAuth()
              if (data.account_name && data.account_name !== accountName) {
                navigate(`/dashboard/${data.account_name}?tab=profile`, { replace: true })
              }
            } else {
              setProfileError((data as { error?: string })?.error || error || 'Failed to update')
            }
          }}
          disabled={profileSaving}
          className="px-4 py-2 bg-blue-600 hover:bg-blue-700 disabled:opacity-50 text-white rounded-lg font-medium"
        >
          {profileSaving ? <i className="fas fa-spinner fa-spin mr-2"></i> : null}
          Save changes
        </button>
        {profileError && <p className="text-red-400 text-sm">{profileError}</p>}
      </div>

      <div className="bg-gray-700/50 rounded-xl p-6 border border-gray-600 mb-8">
        <h3 className="text-sm font-medium text-gray-400 uppercase tracking-wider mb-3">Kingshot ID</h3>
        <p className="text-gray-400 text-sm mb-3">
          Enter your Kingshot player ID to auto-update name, server, and profile picture from the game.
        </p>
        <div className="flex gap-2">
          <input
            type="text"
            value={kingshotIdInput}
            onChange={(e) => setKingshotIdInput(e.target.value)}
            placeholder="Enter Kingshot player ID"
            className="flex-1 px-4 py-2 bg-gray-800 border border-gray-600 rounded-lg text-white focus:border-blue-500 focus:ring-2 focus:ring-blue-500/50 outline-none"
          />
          <button
            onClick={async () => {
              if (!kingshotIdInput.trim()) return
              setKingshotLookingUp(true)
              setKingshotError(null)
              const { ok, data, error } = await api.kingshotLookup(kingshotIdInput.trim())
              setKingshotLookingUp(false)
              if (ok && data?.success) {
                setInGameName(data.in_game_name ?? null)
                setServerNumber(data.server_number ?? null)
                setPlayerId(data.player_id ?? null)
                setProfileEdit((p) => ({
                  ...p,
                  in_game_name: data.in_game_name ?? p.in_game_name,
                  server_number: String(data.server_number ?? p.server_number),
                }))
                await refreshAuth()
              } else {
                setKingshotError((data as { error?: string })?.error || error || 'Lookup failed')
              }
            }}
            disabled={kingshotLookingUp}
            className="px-4 py-2 bg-green-600 hover:bg-green-700 disabled:opacity-50 text-white rounded-lg font-medium"
          >
            {kingshotLookingUp ? <i className="fas fa-spinner fa-spin mr-2"></i> : null}
            Confirm
          </button>
        </div>
        {kingshotError && <p className="text-red-400 text-sm mt-2">{kingshotError}</p>}
        {playerId && <p className="text-gray-500 text-sm mt-2">Current ID: {playerId}</p>}
      </div>

      <div className="bg-gray-700/50 rounded-xl p-6 border border-gray-600">
        <div className="flex items-center justify-between mb-4">
          <h3 className="text-sm font-medium text-gray-400 uppercase tracking-wider">Your Schedule</h3>
          {accountName && serverNumber != null && (
            <span className="text-xs text-green-400 font-medium">Active</span>
          )}
        </div>
        {accountName && serverNumber != null ? (
          <div className="flex items-center justify-between gap-4 p-4 bg-gray-800/50 rounded-lg border border-gray-600">
            <div>
              <p className="font-semibold text-white">{accountName}</p>
              <p className="text-sm text-gray-400">Server {serverNumber}</p>
            </div>
            <Link
              to={`/dashboard/${accountName}?tab=schedule`}
              className="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg text-sm font-medium transition-all flex items-center gap-2"
            >
              <i className="fas fa-external-link-alt"></i>
              Open
            </Link>
          </div>
        ) : (
          <p className="text-gray-500 text-sm">No schedule linked</p>
        )}
      </div>
    </div>
  )
}
