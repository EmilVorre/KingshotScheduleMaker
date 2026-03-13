import { useCallback, useEffect, useState } from 'react'
import { api } from '../../api/client'
import type { AlliancePlayer } from '../../api/client'

interface RedemptionResult {
  player_id: string
  status: string
  message: string
}

interface TabGiftcodeAutomationProps {
  accountName: string | null
  serverNumber: number | null
}

export default function TabGiftcodeAutomation({ accountName, serverNumber }: TabGiftcodeAutomationProps) {
  const [alliances, setAlliances] = useState<
    Array<{ name: string; slug: string; players: AlliancePlayer[]; owner_account: string; owner_server: number }>
  >([])
  const [recipientIds, setRecipientIds] = useState<Set<string>>(new Set())
  const [loading, setLoading] = useState(true)
  const [saving, setSaving] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [giftcodeInput, setGiftcodeInput] = useState('')
  const [redeeming, setRedeeming] = useState(false)
  const [redeemResults, setRedeemResults] = useState<RedemptionResult[] | null>(null)
  const [redeemError, setRedeemError] = useState<string | null>(null)

  const loadData = useCallback(async () => {
    if (!accountName || !serverNumber) return
    setLoading(true)
    setError(null)
    const [alliancesRes, recipientsRes] = await Promise.all([
      api.listAlliances(accountName, serverNumber),
      api.getGiftcodeRecipients(accountName, serverNumber),
    ])
    setLoading(false)
    if (alliancesRes.ok && alliancesRes.data?.alliances) {
      setAlliances(alliancesRes.data.alliances)
    } else {
      setError(alliancesRes.error ?? 'Failed to load alliances')
    }
    if (recipientsRes.ok && recipientsRes.data?.player_ids) {
      setRecipientIds(new Set(recipientsRes.data.player_ids))
    }
  }, [accountName, serverNumber])

  useEffect(() => {
    if (accountName && serverNumber) {
      loadData()
    }
  }, [loadData, accountName, serverNumber])

  const allPlayers = alliances.flatMap((a) => a.players).sort((a, b) => (a.name ?? '').localeCompare(b.name ?? ''))
  const togglePlayer = async (playerId: string) => {
    if (!accountName || !serverNumber || saving) return
    const next = new Set(recipientIds)
    if (next.has(playerId)) {
      next.delete(playerId)
    } else {
      next.add(playerId)
    }
    setRecipientIds(next)
    setSaving(true)
    const { ok } = await api.setGiftcodeRecipients(accountName, serverNumber, Array.from(next))
    setSaving(false)
    if (!ok) {
      setRecipientIds(recipientIds)
    }
  }

  const selectAll = async () => {
    if (!accountName || !serverNumber || saving) return
    const ids = allPlayers.map((p) => p.player_id)
    setRecipientIds(new Set(ids))
    setSaving(true)
    const { ok } = await api.setGiftcodeRecipients(accountName, serverNumber, ids)
    setSaving(false)
    if (!ok) {
      loadData()
    }
  }

  const selectNone = async () => {
    if (!accountName || !serverNumber || saving) return
    setRecipientIds(new Set())
    setSaving(true)
    const { ok } = await api.setGiftcodeRecipients(accountName, serverNumber, [])
    setSaving(false)
    if (!ok) {
      loadData()
    }
  }

  const handleRedeem = async () => {
    if (!accountName || !serverNumber || redeeming) return
    const code = giftcodeInput.trim()
    if (!code) {
      setRedeemError('Enter a gift code')
      return
    }
    if (recipientIds.size === 0) {
      setRedeemError('Select at least one recipient')
      return
    }
    setRedeeming(true)
    setRedeemError(null)
    setRedeemResults(null)
    const { ok, data, error: err } = await api.redeemGiftcode(accountName, serverNumber, code)
    setRedeeming(false)
    if (ok && data?.results) {
      setRedeemResults(data.results)
      setGiftcodeInput('')
    } else {
      setRedeemError(err ?? 'Redemption failed')
    }
  }

  const playerName = (pid: string) =>
    allPlayers.find((p) => p.player_id === pid)?.name ?? pid

  return (
    <div>
      <div className="text-center mb-8">
        <div className="inline-block bg-indigo-900/50 rounded-full p-4 mb-4">
          <i className="fas fa-gift text-indigo-400 text-3xl"></i>
        </div>
        <h2 className="text-3xl font-bold text-white mb-2">Giftcode Automation</h2>
        <p className="text-gray-400">
          Choose which alliance players get gift codes redeemed. New codes are fetched and redeemed automatically every 5 minutes. You can also redeem a code manually.
        </p>
      </div>

      {/* Redeem section - show when we have recipients */}
      {!loading && recipientIds.size > 0 && (
        <div className="bg-gray-800 rounded-lg shadow-xl p-6 border border-gray-700 mb-6">
          <h3 className="text-lg font-semibold text-white mb-4">Redeem gift codes</h3>
          <div className="flex flex-wrap gap-3 items-end">
            <div className="flex-1 min-w-[200px]">
              <label className="block text-sm text-gray-400 mb-1">Gift code</label>
              <input
                type="text"
                value={giftcodeInput}
                onChange={(e) => setGiftcodeInput(e.target.value)}
                placeholder="Enter gift code"
                disabled={redeeming}
                className="w-full px-4 py-2 bg-gray-700 border border-gray-600 rounded-lg text-white placeholder-gray-500 focus:ring-2 focus:ring-indigo-500 focus:border-indigo-500"
              />
            </div>
            <button
              onClick={handleRedeem}
              disabled={redeeming || !giftcodeInput.trim()}
              className="px-4 py-2 bg-indigo-600 hover:bg-indigo-500 disabled:opacity-50 text-white rounded-lg font-medium"
            >
              {redeeming ? (
                <>
                  <i className="fas fa-spinner fa-spin mr-2"></i>Redeeming...
                </>
              ) : (
                <>
                  <i className="fas fa-gift mr-2"></i>Redeem
                </>
              )}
            </button>
          </div>
          <p className="text-gray-500 text-sm mt-2">
            Enter a code to redeem for {recipientIds.size} selected player(s).
          </p>
          {redeemError && (
            <div className="mt-3 p-3 bg-red-900/50 border border-red-700 rounded-lg text-red-200 text-sm">
              <i className="fas fa-exclamation-circle mr-2"></i>{redeemError}
            </div>
          )}
          {redeemResults && redeemResults.length > 0 && (
            <div className="mt-4">
              <h4 className="text-sm font-medium text-gray-400 mb-2">Redemption results</h4>
              <div className="space-y-1 max-h-40 overflow-y-auto">
                {redeemResults.map((r, i) => (
                  <div key={i} className="flex justify-between text-sm">
                    <span className="text-gray-300">{playerName(r.player_id)}</span>
                    <span className={r.status === 'SUCCESS' || r.status === 'RECEIVED' || r.status === 'SAME_TYPE_EXCHANGE' ? 'text-green-400' : 'text-amber-400'}>
                      {r.message}
                    </span>
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>
      )}

      {loading && (
        <div className="bg-gray-800 rounded-lg shadow-xl p-12 text-center border border-gray-700">
          <i className="fas fa-spinner fa-spin text-4xl text-indigo-400 mb-4"></i>
          <p className="text-gray-400">Loading...</p>
        </div>
      )}

      {!loading && error && (
        <div className="bg-red-900/50 border-l-4 border-red-500 text-red-200 p-4 rounded-lg mb-6">
          <i className="fas fa-exclamation-circle mr-2"></i>
          {error}
        </div>
      )}

      {!loading && !error && allPlayers.length === 0 && (
        <div className="bg-gray-800 rounded-lg shadow-xl p-12 text-center border border-gray-700">
          <i className="fas fa-users text-4xl text-gray-500 mb-4"></i>
          <p className="text-xl text-gray-400 mb-2">No alliance players yet</p>
          <p className="text-gray-500 text-sm">
            Add players to your alliance in the Alliance Organisation tab first.
          </p>
        </div>
      )}

      {!loading && !error && allPlayers.length > 0 && (
        <div className="bg-gray-800 rounded-lg shadow-xl p-6 border border-gray-700">
          <div className="flex items-center justify-between mb-4">
            <h3 className="text-lg font-semibold text-white">Auto-redeem gift codes for</h3>
            <div className="flex gap-2">
              <button
                onClick={selectAll}
                disabled={saving}
                className="px-3 py-1.5 text-sm bg-indigo-600 hover:bg-indigo-500 disabled:opacity-50 text-white rounded-lg"
              >
                Select all
              </button>
              <button
                onClick={selectNone}
                disabled={saving}
                className="px-3 py-1.5 text-sm bg-gray-600 hover:bg-gray-500 disabled:opacity-50 text-white rounded-lg"
              >
                Select none
              </button>
            </div>
          </div>
          <p className="text-gray-500 text-sm mb-4">
            Check the players who should automatically receive gift codes when the system redeems them.
          </p>
          <div className="flex flex-wrap gap-3">
            {allPlayers.map((player) => (
              <label
                key={player.player_id}
                className="flex items-center gap-3 px-4 py-3 bg-gray-700/50 rounded-lg border border-gray-600 hover:border-indigo-500/50 cursor-pointer transition-all"
              >
                <input
                  type="checkbox"
                  checked={recipientIds.has(player.player_id)}
                  onChange={() => togglePlayer(player.player_id)}
                  disabled={saving}
                  className="w-4 h-4 rounded border-gray-500 text-indigo-600 focus:ring-indigo-500"
                />
                {player.avatar_image ? (
                  <img
                    src={player.avatar_image}
                    alt=""
                    className="w-10 h-10 rounded-full object-cover"
                  />
                ) : (
                  <div className="w-10 h-10 rounded-full bg-indigo-600 flex items-center justify-center text-white font-semibold">
                    {player.name.charAt(0).toUpperCase()}
                  </div>
                )}
                <div>
                  <p className="font-medium text-white">{player.name}</p>
                  <p className="text-xs text-gray-400">ID: {player.player_id}</p>
                </div>
              </label>
            ))}
          </div>
          {saving && (
            <p className="text-indigo-400 text-sm mt-4">
              <i className="fas fa-spinner fa-spin mr-2"></i>Saving...
            </p>
          )}
        </div>
      )}
    </div>
  )
}
