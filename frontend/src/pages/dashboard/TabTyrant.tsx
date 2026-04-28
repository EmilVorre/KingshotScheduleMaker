import { useCallback, useEffect, useState } from 'react'
import { api } from '../../api/client'

interface TabTyrantProps {
  accountName: string | null
  serverNumber: number | null
}

type WsRow = {
  id: string
  display_name: string
  kingshot_server_number: number
}

export default function TabTyrant({ accountName, serverNumber }: TabTyrantProps) {
  const [workspaces, setWorkspaces] = useState<WsRow[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  const [selectedId, setSelectedId] = useState<string | null>(null)

  const [sortMode, setSortMode] = useState<'level_then_tg' | 'tg_then_level'>('level_then_tg')

  type SubRow = {
    player_id?: string
    payload?: Record<string, unknown>
    created_at?: string
    rank_min_level?: number
    rank_min_tg?: number
    id?: string
  }

  const [subs, setSubs] = useState<SubRow[]>([])
  const [subsLoading, setSubsLoading] = useState(false)

  const loadWs = useCallback(async () => {
    if (!accountName || serverNumber == null) return
    setLoading(true)
    setError(null)
    const r = await api.listServerOrgWorkspaces(accountName, serverNumber)
    setLoading(false)
    if (r.ok && r.data?.success && r.data.workspaces) {
      const list = r.data.workspaces as WsRow[]
      setWorkspaces(list)
      setSelectedId((prev) => (prev && list.some((x) => x.id === prev) ? prev : list[0]?.id ?? null))
    } else {
      setError(r.error ?? 'Failed to load')
    }
  }, [accountName, serverNumber])

  const loadSubs = useCallback(async () => {
    if (!accountName || serverNumber == null || !selectedId) return
    setSubsLoading(true)
    const r = await api.listTyrantSubmissions(accountName, serverNumber, selectedId, sortMode)
    setSubsLoading(false)
    if (r.ok && r.data?.success && Array.isArray(r.data.submissions)) {
      setSubs(r.data.submissions as SubRow[])
    }
  }, [accountName, serverNumber, selectedId, sortMode])

  useEffect(() => {
    loadWs()
  }, [loadWs])

  useEffect(() => {
    loadSubs()
  }, [loadSubs])

  if (!accountName || serverNumber == null) {
    return <p className="text-gray-400">Missing account.</p>
  }

  return (
    <div className="max-w-5xl mx-auto space-y-6">
      <div className="text-center mb-6">
        <div className="inline-block bg-cyan-900/50 rounded-full p-4 mb-4">
          <i className="fas fa-dragon text-cyan-400 text-3xl"></i>
        </div>
        <h2 className="text-3xl font-bold text-white mb-2">Tyrant registrations</h2>
        <p className="text-gray-400">
          Ranking uses the weakest troop across archer/cavalry/infantry (minimum tier wins). Duplicate player ids keep latest
          submission.
        </p>
      </div>

      {loading && (
        <p className="text-gray-400 text-center">
          <i className="fas fa-spinner fa-spin mr-2"></i>
          Loading workspaces…
        </p>
      )}
      {error && <div className="bg-red-900/50 border border-red-600/60 text-red-200 px-4 py-3 rounded-lg">{error}</div>}

      {workspaces.length > 0 && (
        <>
          <div className="flex flex-wrap gap-4 items-end justify-between">
            <div>
              <label className="block text-xs text-gray-400 mb-1">Workspace</label>
              <select
                className="px-4 py-2 rounded-lg bg-gray-800 border border-gray-600 text-white min-w-[220px]"
                value={selectedId ?? ''}
                onChange={(e) => setSelectedId(e.target.value || null)}
              >
                {workspaces.map((w) => (
                  <option key={w.id} value={w.id}>
                    {w.display_name}
                  </option>
                ))}
              </select>
            </div>
            <div>
              <label className="block text-xs text-gray-400 mb-1">Sort priority</label>
              <select
                className="px-4 py-2 rounded-lg bg-gray-800 border border-gray-600 text-white"
                value={sortMode}
                onChange={(e) => setSortMode(e.target.value as 'level_then_tg' | 'tg_then_level')}
              >
                <option value="level_then_tg">Troop level bands first</option>
                <option value="tg_then_level">TG bands first</option>
              </select>
            </div>
          </div>

          {subsLoading ? (
            <p className="text-gray-400">
              <i className="fas fa-spinner fa-spin mr-2"></i>
              Loading submissions…
            </p>
          ) : (
            <div className="overflow-x-auto rounded-xl border border-gray-700 bg-gray-800">
              <table className="w-full text-sm text-left text-gray-200">
                <thead className="bg-gray-900/90 text-xs uppercase text-gray-400">
                  <tr>
                    <th className="px-4 py-3">#</th>
                    <th className="px-4 py-3">Player id</th>
                    <th className="px-4 py-3">Alliance</th>
                    <th className="px-4 py-3">Full 5h</th>
                    <th className="px-4 py-3">Ranks min L / TG</th>
                    <th className="px-4 py-3">Archer</th>
                    <th className="px-4 py-3">Cavalry</th>
                    <th className="px-4 py-3">Infantry</th>
                    <th className="px-4 py-3">Submitted</th>
                  </tr>
                </thead>
                <tbody>
                  {subs.map((row, i) => {
                    const p = (row.payload ?? {}) as {
                      alliance?: string
                      participate_full_five_hours?: boolean
                      archer?: { level_band?: string; tg_band?: string }
                      cavalry?: { level_band?: string; tg_band?: string }
                      infantry?: { level_band?: string; tg_band?: string }
                    }
                    const troop = (a?: { level_band?: string; tg_band?: string }) =>
                      `${a?.level_band ?? '?'} · ${a?.tg_band ?? '?'}`
                    return (
                      <tr key={`${row.id ?? row.player_id}-${row.created_at}`} className="border-t border-gray-700 hover:bg-gray-800/50">
                        <td className="px-4 py-2">{i + 1}</td>
                        <td className="px-4 py-2 font-mono">{row.player_id}</td>
                        <td className="px-4 py-2">{p.alliance ?? '—'}</td>
                        <td className="px-4 py-2 text-center">{p.participate_full_five_hours ? 'Yes' : '—'}</td>
                        <td className="px-4 py-2">
                          {row.rank_min_level ?? '—'} / {row.rank_min_tg ?? '—'}
                        </td>
                        <td className="px-4 py-2 text-xs">{troop(p.archer)}</td>
                        <td className="px-4 py-2 text-xs">{troop(p.cavalry)}</td>
                        <td className="px-4 py-2 text-xs">{troop(p.infantry)}</td>
                        <td className="px-4 py-2 text-xs text-gray-400">{row.created_at ?? '—'}</td>
                      </tr>
                    )
                  })}
                  {subs.length === 0 && (
                    <tr>
                      <td colSpan={9} className="px-4 py-8 text-center text-gray-500">
                        No submissions yet for this workspace.
                      </td>
                    </tr>
                  )}
                </tbody>
              </table>
            </div>
          )}
        </>
      )}

      {!loading && workspaces.length === 0 && (
        <p className="text-center text-gray-500">
          Create a workspace in &quot;Manage server&quot; first (Server Organisation sidebar).
        </p>
      )}
    </div>
  )
}
