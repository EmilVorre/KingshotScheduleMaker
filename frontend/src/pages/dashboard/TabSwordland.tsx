import { useCallback, useEffect, useRef, useState } from 'react'
import { api } from '../../api/client'
import type { AlliancePlayer } from '../../api/client'

interface Legion {
  name: string
  member_ids: string[]
  filler_ids?: string[]
}

interface AttendanceRecord {
  id: string
  date: string
  label?: string
  legion_1: { attended: string[]; absent: string[]; filler?: string[] }
  legion_2: { attended: string[]; absent: string[]; filler?: string[] }
}

interface TabSwordlandProps {
  accountName: string | null
  serverNumber: number | null
}

export default function TabSwordland({ accountName, serverNumber }: TabSwordlandProps) {
  const [alliances, setAlliances] = useState<
    Array<{ name: string; slug: string; players: AlliancePlayer[]; owner_account: string; owner_server: number; is_owner?: boolean }>
  >([])
  const [selectedAllianceKey, setSelectedAllianceKey] = useState<string | null>(null)
  const initialSelectionSet = useRef(false)
  const [legions, setLegions] = useState<Legion[]>([
    { name: 'Legion 1', member_ids: [], filler_ids: [] },
    { name: 'Legion 2', member_ids: [], filler_ids: [] },
  ])
  const [attendanceRecords, setAttendanceRecords] = useState<AttendanceRecord[]>([])
  const [loading, setLoading] = useState(true)
  const [saving, setSaving] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [showAttendanceModal, setShowAttendanceModal] = useState(false)
  const [editingRecordId, setEditingRecordId] = useState<string | null>(null)
  const [attendanceDate, setAttendanceDate] = useState(new Date().toISOString().slice(0, 10))
  const [attendanceLabel, setAttendanceLabel] = useState('')
  const [attendanceLegion1, setAttendanceLegion1] = useState<Record<string, boolean>>({})
  const [attendanceLegion2, setAttendanceLegion2] = useState<Record<string, boolean>>({})
  const [submittingAttendance, setSubmittingAttendance] = useState(false)
  const [playersView, setPlayersView] = useState<'cards' | 'table'>('cards')

  const selectedAlliance = selectedAllianceKey
    ? alliances.find((a) => `${a.owner_account}:${a.owner_server}:${a.slug}` === selectedAllianceKey)
    : null
  const allPlayers = (selectedAlliance?.players ?? []).sort((a, b) => (a.name ?? '').localeCompare(b.name ?? ''))
  const playerName = (pid: string) => allPlayers.find((p) => p.player_id === pid)?.name ?? pid

  const loadData = useCallback(async () => {
    if (!accountName || !serverNumber) return
    setLoading(true)
    setError(null)
    const alliancesRes = await api.listAlliances(accountName, serverNumber)
    setLoading(false)
    if (alliancesRes.ok && alliancesRes.data?.alliances) {
      const alls = alliancesRes.data.alliances
      setAlliances(alls)
      if (alls.length > 0) {
        if (!initialSelectionSet.current) {
          const first = alls.find((a) => a.is_owner) ?? alls[0]
          setSelectedAllianceKey(`${first.owner_account}:${first.owner_server}:${first.slug}`)
          initialSelectionSet.current = true
        }
      } else {
        setSelectedAllianceKey(null)
        initialSelectionSet.current = false
      }
    } else {
      setError(alliancesRes.error ?? 'Failed to load alliances')
    }
  }, [accountName, serverNumber])

  useEffect(() => {
    initialSelectionSet.current = false
  }, [accountName, serverNumber])

  const loadSwordlandData = useCallback(async () => {
    if (!selectedAlliance) return
    const swordlandRes = await api.getSwordland(
      selectedAlliance.owner_account,
      selectedAlliance.owner_server,
      selectedAlliance.slug
    )
    if (swordlandRes.ok && swordlandRes.data) {
      const legs = (swordlandRes.data.legions ?? [
        { name: 'Legion 1', member_ids: [], filler_ids: [] },
        { name: 'Legion 2', member_ids: [], filler_ids: [] },
      ]).map((l) => ({ ...l, filler_ids: l.filler_ids ?? [] }))
      setLegions(legs.length >= 2 ? legs : [legs[0] ?? { name: 'Legion 1', member_ids: [], filler_ids: [] }, { name: 'Legion 2', member_ids: [], filler_ids: [] }])
      setAttendanceRecords(swordlandRes.data.attendance_records ?? [])
    }
  }, [selectedAlliance])

  useEffect(() => {
    if (selectedAlliance) loadSwordlandData()
    else {
      setLegions([
        { name: 'Legion 1', member_ids: [], filler_ids: [] },
        { name: 'Legion 2', member_ids: [], filler_ids: [] },
      ])
      setAttendanceRecords([])
    }
  }, [selectedAlliance, loadSwordlandData])

  useEffect(() => {
    if (accountName && serverNumber) loadData()
  }, [loadData, accountName, serverNumber])

  const assignToLegion = async (playerId: string, legionIndex: number | null) => {
    if (!selectedAlliance || saving) return
    const next: Legion[] = [
      {
        ...legions[0],
        member_ids: legions[0].member_ids.filter((id) => id !== playerId),
        filler_ids: (legions[0].filler_ids ?? []).filter((id) => id !== playerId),
      },
      {
        ...legions[1],
        member_ids: legions[1].member_ids.filter((id) => id !== playerId),
        filler_ids: (legions[1].filler_ids ?? []).filter((id) => id !== playerId),
      },
    ]
    if (legionIndex !== null) {
      if (legionIndex === 0 || legionIndex === 1) {
        next[legionIndex].member_ids.push(playerId)
      } else {
        next[legionIndex - 2].filler_ids = [...(next[legionIndex - 2].filler_ids ?? []), playerId]
      }
    }
    setLegions(next)
    setSaving(true)
    const { ok } = await api.setSwordlandLegions(
      selectedAlliance.owner_account,
      selectedAlliance.owner_server,
      selectedAlliance.slug,
      next
    )
    setSaving(false)
    if (!ok) loadSwordlandData()
  }

  const resetLegions = async () => {
    if (!selectedAlliance || saving) return
    if (!window.confirm('Reset all players to None? This will remove everyone from both legions.')) return
    const next: Legion[] = [
      { ...legions[0], member_ids: [], filler_ids: [] },
      { ...legions[1], member_ids: [], filler_ids: [] },
    ]
    setLegions(next)
    setSaving(true)
    const { ok } = await api.setSwordlandLegions(
      selectedAlliance.owner_account,
      selectedAlliance.owner_server,
      selectedAlliance.slug,
      next
    )
    setSaving(false)
    if (!ok) loadSwordlandData()
  }

  const getPlayerLegion = (playerId: string): number | null => {
    if (legions[0]?.member_ids.includes(playerId)) return 0
    if (legions[1]?.member_ids.includes(playerId)) return 1
    if ((legions[0]?.filler_ids ?? []).includes(playerId)) return 2
    if ((legions[1]?.filler_ids ?? []).includes(playerId)) return 3
    return null
  }

  const getPlayerAttendanceCounts = (playerId: string): { attended: number; absent: number } => {
    let attended = 0
    let absent = 0
    const filler1Ids = legions[0]?.filler_ids ?? []
    const filler2Ids = legions[1]?.filler_ids ?? []
    for (const r of attendanceRecords) {
      const inL1 = r.legion_1.attended.includes(playerId) || r.legion_1.absent.includes(playerId)
      const inL2 = r.legion_2.attended.includes(playerId) || r.legion_2.absent.includes(playerId)
      if (!inL1 && !inL2) continue
      const filler1 = (r.legion_1.filler && r.legion_1.filler.length > 0) ? r.legion_1.filler : filler1Ids
      const filler2 = (r.legion_2.filler && r.legion_2.filler.length > 0) ? r.legion_2.filler : filler2Ids
      if (inL1 ? r.legion_1.attended.includes(playerId) : r.legion_2.attended.includes(playerId)) {
        attended += 1
      } else {
        const wasFiller = inL1 ? filler1.includes(playerId) : filler2.includes(playerId)
        if (!wasFiller) absent += 1
      }
    }
    return { attended, absent }
  }

  const openAttendanceModal = () => {
    setEditingRecordId(null)
    const l1: Record<string, boolean> = {}
    ;[...(legions[0]?.member_ids ?? []), ...(legions[0]?.filler_ids ?? [])].forEach((id) => (l1[id] = false))
    const l2: Record<string, boolean> = {}
    ;[...(legions[1]?.member_ids ?? []), ...(legions[1]?.filler_ids ?? [])].forEach((id) => (l2[id] = false))
    setAttendanceLegion1(l1)
    setAttendanceLegion2(l2)
    setAttendanceDate(new Date().toISOString().slice(0, 10))
    setAttendanceLabel('')
    setShowAttendanceModal(true)
  }

  const openEditAttendanceModal = (r: AttendanceRecord) => {
    setEditingRecordId(r.id)
    setAttendanceDate(r.date)
    setAttendanceLabel(r.label ?? '')
    const l1Ids = [...new Set([...(legions[0]?.member_ids ?? []), ...(legions[0]?.filler_ids ?? []), ...r.legion_1.attended, ...r.legion_1.absent])]
    const l2Ids = [...new Set([...(legions[1]?.member_ids ?? []), ...(legions[1]?.filler_ids ?? []), ...r.legion_2.attended, ...r.legion_2.absent])]
    const l1: Record<string, boolean> = {}
    l1Ids.forEach((id) => (l1[id] = r.legion_1.attended.includes(id)))
    const l2: Record<string, boolean> = {}
    l2Ids.forEach((id) => (l2[id] = r.legion_2.attended.includes(id)))
    setAttendanceLegion1(l1)
    setAttendanceLegion2(l2)
    setShowAttendanceModal(true)
  }

  const toggleAttendance = (legionIndex: number, playerId: string) => {
    if (legionIndex === 0) {
      setAttendanceLegion1((prev) => ({ ...prev, [playerId]: !prev[playerId] }))
    } else {
      setAttendanceLegion2((prev) => ({ ...prev, [playerId]: !prev[playerId] }))
    }
  }

  const submitAttendance = async () => {
    if (!selectedAlliance || submittingAttendance) return
    setSubmittingAttendance(true)
    const legion1Attended = Object.entries(attendanceLegion1).filter(([, v]) => v).map(([k]) => k)
    const legion1Absent = Object.entries(attendanceLegion1)
      .filter(([, v]) => !v)
      .map(([k]) => k)
      .filter((id) => !(legions[0]?.filler_ids ?? []).includes(id))
    const legion2Attended = Object.entries(attendanceLegion2).filter(([, v]) => v).map(([k]) => k)
    const legion2Absent = Object.entries(attendanceLegion2)
      .filter(([, v]) => !v)
      .map(([k]) => k)
      .filter((id) => !(legions[1]?.filler_ids ?? []).includes(id))
    const legion1Filler = legions[0]?.filler_ids ?? []
    const legion2Filler = legions[1]?.filler_ids ?? []
    const payload = {
      date: attendanceDate,
      label: attendanceLabel.trim() || undefined,
      legion_1_attended: legion1Attended,
      legion_1_absent: legion1Absent,
      legion_1_filler: legion1Filler,
      legion_2_attended: legion2Attended,
      legion_2_absent: legion2Absent,
      legion_2_filler: legion2Filler,
    }
    const { ok } = editingRecordId
      ? await api.updateSwordlandAttendance(
          selectedAlliance.owner_account,
          selectedAlliance.owner_server,
          selectedAlliance.slug,
          editingRecordId,
          payload
        )
      : await api.addSwordlandAttendance(
          selectedAlliance.owner_account,
          selectedAlliance.owner_server,
          selectedAlliance.slug,
          payload
        )
    setSubmittingAttendance(false)
    if (ok) {
      setShowAttendanceModal(false)
      setEditingRecordId(null)
      loadSwordlandData()
    }
  }

  return (
    <div>
      <div className="text-center mb-8">
        <div className="inline-block bg-indigo-900/50 rounded-full p-4 mb-4">
          <i className="fas fa-landmark text-indigo-400 text-3xl"></i>
        </div>
        <h2 className="text-3xl font-bold text-white mb-2">Swordland</h2>
        <p className="text-gray-400">
          Assign alliance members to two legions and track attendance for each event.
        </p>
      </div>

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

      {!loading && !error && (
        <div className="space-y-6">
          {/* Alliance selector + Player cards */}
          <div className="bg-gray-800 rounded-lg shadow-xl p-6 border border-gray-700">
            {alliances.length > 1 && (
              <div className="mb-4">
                <label className="block text-sm text-gray-400 mb-2">Alliance</label>
                <select
                  value={selectedAllianceKey ?? ''}
                  onChange={(e) => setSelectedAllianceKey(e.target.value || null)}
                  className="px-3 py-2 bg-gray-700 border border-gray-600 rounded-lg text-white"
                >
                  {alliances.map((a) => (
                    <option key={`${a.owner_account}:${a.owner_server}:${a.slug}`} value={`${a.owner_account}:${a.owner_server}:${a.slug}`}>
                      {a.name}
                      {!a.is_owner && ' (shared)'}
                    </option>
                  ))}
                </select>
                <p className="text-xs text-gray-500 mt-1">Legion assignments are shared across all members of this alliance.</p>
              </div>
            )}
            <div className="flex items-center justify-between gap-4 mb-4 flex-wrap">
              <h3 className="text-lg font-semibold text-white">Players</h3>
              <div className="flex items-center gap-2">
                <span className="text-xs text-gray-500">View:</span>
                <div className="flex rounded-lg overflow-hidden border border-gray-600">
                  <button
                    onClick={() => setPlayersView('cards')}
                    className={`px-3 py-1.5 text-xs ${playersView === 'cards' ? 'bg-indigo-600 text-white' : 'bg-gray-600 text-gray-400 hover:bg-gray-500'}`}
                    title="Card view"
                  >
                    <i className="fas fa-th-large mr-1"></i>Cards
                  </button>
                  <button
                    onClick={() => setPlayersView('table')}
                    className={`px-3 py-1.5 text-xs ${playersView === 'table' ? 'bg-indigo-600 text-white' : 'bg-gray-600 text-gray-400 hover:bg-gray-500'}`}
                    title="Table view"
                  >
                    <i className="fas fa-table mr-1"></i>Table
                  </button>
                </div>
                <button
                  onClick={resetLegions}
                  disabled={
                    saving ||
                    !selectedAlliance ||
                    ((legions[0]?.member_ids?.length ?? 0) + (legions[0]?.filler_ids?.length ?? 0) === 0 &&
                      (legions[1]?.member_ids?.length ?? 0) + (legions[1]?.filler_ids?.length ?? 0) === 0)
                  }
                  className="px-3 py-1.5 text-sm bg-gray-600 hover:bg-gray-500 disabled:bg-gray-700 disabled:cursor-not-allowed disabled:opacity-50 text-gray-300 rounded-lg"
                >
                  <i className="fas fa-undo mr-2"></i>Reset all to None
                </button>
              </div>
            </div>
            <p className="text-gray-500 text-sm mb-4">
              Assign each player to one legion. Filler = assigned but not expected; if they don&apos;t show, it won&apos;t count against them.
            </p>
            {allPlayers.length === 0 ? (
              <p className="text-gray-500">Add players to your alliance in Alliance Organisation first.</p>
            ) : playersView === 'table' ? (
              <div className="overflow-x-auto rounded-lg border border-gray-600">
                <table className="w-full text-sm">
                  <thead>
                    <tr className="bg-gray-700/80 text-left">
                      <th className="px-4 py-3 text-gray-400 font-medium">Player</th>
                      <th className="px-4 py-3 text-gray-400 font-medium">Legion</th>
                      <th className="px-4 py-3 text-gray-400 font-medium">Attendance</th>
                    </tr>
                  </thead>
                  <tbody>
                    {allPlayers.map((player) => {
                      const legionIdx = getPlayerLegion(player.player_id)
                      const attendance = getPlayerAttendanceCounts(player.player_id)
                      return (
                        <tr key={player.player_id} className="border-t border-gray-600 hover:bg-gray-700/30">
                          <td className="px-4 py-3">
                            <div className="flex items-center gap-3">
                              {player.avatar_image ? (
                                <img src={player.avatar_image} alt="" className="w-8 h-8 rounded-full object-cover" />
                              ) : (
                                <div className="w-8 h-8 rounded-full bg-indigo-600 flex items-center justify-center text-white text-xs font-semibold">
                                  {player.name.charAt(0).toUpperCase()}
                                </div>
                              )}
                              <div>
                                <p className="font-medium text-white">{player.name}</p>
                                <p className="text-xs text-gray-500">ID: {player.player_id}</p>
                              </div>
                            </div>
                          </td>
                          <td className="px-4 py-3">
                            <div className="flex flex-wrap gap-1">
                              {[0, 2, 1, 3, null].map((idx) => (
                                <button
                                  key={idx ?? 'n'}
                                  onClick={() => assignToLegion(player.player_id, idx)}
                                  disabled={saving}
                                  className={`px-2 py-1 text-xs rounded ${
                                    legionIdx === idx
                                      ? idx === null
                                        ? 'bg-gray-500 text-white'
                                        : idx === 2 || idx === 3
                                          ? 'bg-amber-600 text-white'
                                          : 'bg-indigo-600 text-white'
                                      : 'bg-gray-600 text-gray-300 hover:bg-gray-500'
                                  }`}
                                >
                                  {idx === 0 ? 'L1' : idx === 2 ? 'L1 filler' : idx === 1 ? 'L2' : idx === 3 ? 'L2 filler' : 'None'}
                                </button>
                              ))}
                            </div>
                          </td>
                          <td className="px-4 py-3">
                            {(attendance.attended > 0 || attendance.absent > 0) ? (
                              <span>
                                <span className="text-green-400">{attendance.attended}✓</span>
                                <span className="text-gray-500 mx-1">·</span>
                                <span className="text-red-400">{attendance.absent}✗</span>
                              </span>
                            ) : (
                              <span className="text-gray-500">—</span>
                            )}
                          </td>
                        </tr>
                      )
                    })}
                  </tbody>
                </table>
              </div>
            ) : (
              <div className="grid sm:grid-cols-2 lg:grid-cols-3 gap-4">
                {allPlayers.map((player) => {
                  const legionIdx = getPlayerLegion(player.player_id)
                  const attendance = getPlayerAttendanceCounts(player.player_id)
                  return (
                    <div
                      key={player.player_id}
                      className="bg-gray-700/50 rounded-lg p-4 border border-gray-600 hover:border-indigo-500/50 transition-all"
                    >
                      <div className="flex items-center gap-3 mb-3">
                        {player.avatar_image ? (
                          <img src={player.avatar_image} alt="" className="w-12 h-12 rounded-full object-cover" />
                        ) : (
                          <div className="w-12 h-12 rounded-full bg-indigo-600 flex items-center justify-center text-white font-semibold">
                            {player.name.charAt(0).toUpperCase()}
                          </div>
                        )}
                        <div className="flex-1 min-w-0">
                          <p className="font-medium text-white truncate">{player.name}</p>
                          <p className="text-xs text-gray-400">ID: {player.player_id}</p>
                        </div>
                      </div>
                      <div className="mb-3">
                        <label className="block text-xs text-gray-500 mb-1">Legion</label>
                        <div className="flex flex-wrap gap-1">
                          <button
                            onClick={() => assignToLegion(player.player_id, 0)}
                            disabled={saving}
                            className={`px-2 py-1 text-xs rounded ${
                              legionIdx === 0 ? 'bg-indigo-600 text-white' : 'bg-gray-600 text-gray-300 hover:bg-gray-500'
                            }`}
                          >
                            L1
                          </button>
                          <button
                            onClick={() => assignToLegion(player.player_id, 2)}
                            disabled={saving}
                            className={`px-2 py-1 text-xs rounded ${
                              legionIdx === 2 ? 'bg-amber-600 text-white' : 'bg-gray-600 text-gray-300 hover:bg-gray-500'
                            }`}
                            title="Legion 1 filler - assigned but not expected"
                          >
                            L1 filler
                          </button>
                          <button
                            onClick={() => assignToLegion(player.player_id, 1)}
                            disabled={saving}
                            className={`px-2 py-1 text-xs rounded ${
                              legionIdx === 1 ? 'bg-indigo-600 text-white' : 'bg-gray-600 text-gray-300 hover:bg-gray-500'
                            }`}
                          >
                            L2
                          </button>
                          <button
                            onClick={() => assignToLegion(player.player_id, 3)}
                            disabled={saving}
                            className={`px-2 py-1 text-xs rounded ${
                              legionIdx === 3 ? 'bg-amber-600 text-white' : 'bg-gray-600 text-gray-300 hover:bg-gray-500'
                            }`}
                            title="Legion 2 filler - assigned but not expected"
                          >
                            L2 filler
                          </button>
                          <button
                            onClick={() => assignToLegion(player.player_id, null)}
                            disabled={saving}
                            className={`px-2 py-1 text-xs rounded ${
                              legionIdx === null ? 'bg-gray-500 text-white' : 'bg-gray-600 text-gray-400 hover:bg-gray-500'
                            }`}
                          >
                            None
                          </button>
                        </div>
                      </div>
                      {(attendance.attended > 0 || attendance.absent > 0) && (
                        <div>
                          <label className="block text-xs text-gray-500 mb-1">Attendance</label>
                          <p className="text-sm">
                            <span className="text-green-400">{attendance.attended} showed up</span>
                            <span className="text-gray-500 mx-1">·</span>
                            <span className="text-red-400">{attendance.absent} not shown up</span>
                          </p>
                        </div>
                      )}
                    </div>
                  )
                })}
              </div>
            )}
            {saving && (
              <p className="text-indigo-400 text-sm mt-4">
                <i className="fas fa-spinner fa-spin mr-2"></i>Saving...
              </p>
            )}
          </div>

          {/* Record attendance */}
          <div className="bg-gray-800 rounded-lg shadow-xl p-6 border border-gray-700">
            <h3 className="text-lg font-semibold text-white mb-4">Attendance</h3>
            <button
              onClick={openAttendanceModal}
              disabled={
                !selectedAlliance ||
                ((legions[0]?.member_ids?.length ?? 0) + (legions[0]?.filler_ids?.length ?? 0) === 0 &&
                  (legions[1]?.member_ids?.length ?? 0) + (legions[1]?.filler_ids?.length ?? 0) === 0)
              }
              className="px-4 py-2 bg-indigo-600 hover:bg-indigo-500 disabled:bg-gray-600 disabled:cursor-not-allowed text-white rounded-lg font-medium"
            >
              <i className="fas fa-clipboard-check mr-2"></i>Record attendance
            </button>
            <p className="text-gray-500 text-sm mt-2">
              Mark who showed up and who did not for each legion. Assign members to legions first.
            </p>

            {attendanceRecords.length > 0 && (
              <div className="mt-6">
                <h4 className="text-sm font-medium text-gray-400 mb-2">Past records</h4>
                <div className="space-y-2 max-h-60 overflow-y-auto">
                  {[...attendanceRecords].reverse().map((r) => (
                    <div
                      key={r.id}
                      className="flex items-center justify-between py-2 px-3 bg-gray-700/50 rounded border border-gray-600 gap-2"
                    >
                      <span className="text-gray-300 flex-1 min-w-0">
                        {r.date}
                        {r.label && <span className="text-gray-500 ml-2">({r.label})</span>}
                      </span>
                      <span className="text-sm text-gray-400 shrink-0">
                        L1: {r.legion_1.attended.length}✓ {r.legion_1.absent.length}✗ · L2: {r.legion_2.attended.length}✓{' '}
                        {r.legion_2.absent.length}✗
                      </span>
                      <button
                        onClick={() => openEditAttendanceModal(r)}
                        className="px-2 py-1 text-xs bg-gray-600 hover:bg-gray-500 text-gray-300 rounded shrink-0"
                        title="Edit"
                      >
                        <i className="fas fa-edit"></i>
                      </button>
                    </div>
                  ))}
                </div>
              </div>
            )}
          </div>
        </div>
      )}

      {/* Attendance modal */}
      {showAttendanceModal && (
        <div className="fixed inset-0 bg-black/70 flex items-center justify-center z-50 p-4">
          <div className="bg-gray-800 rounded-xl shadow-2xl border border-gray-700 max-w-2xl w-full max-h-[90vh] overflow-y-auto">
            <div className="p-6">
              <h3 className="text-xl font-bold text-white mb-4">Record attendance</h3>
              <div className="space-y-4 mb-6">
                <div>
                  <label className="block text-sm text-gray-400 mb-1">Date</label>
                  <input
                    type="date"
                    value={attendanceDate}
                    onChange={(e) => setAttendanceDate(e.target.value)}
                    className="w-full px-4 py-2 bg-gray-700 border border-gray-600 rounded-lg text-white"
                  />
                </div>
                <div>
                  <label className="block text-sm text-gray-400 mb-1">Label (optional)</label>
                  <input
                    type="text"
                    value={attendanceLabel}
                    onChange={(e) => setAttendanceLabel(e.target.value)}
                    placeholder="e.g. Swordland Week 1"
                    className="w-full px-4 py-2 bg-gray-700 border border-gray-600 rounded-lg text-white placeholder-gray-500"
                  />
                </div>
              </div>

              <div className="grid md:grid-cols-2 gap-6 mb-6">
                {[0, 1].map((idx) => {
                  const memberIds = idx === 0 ? Object.keys(attendanceLegion1) : Object.keys(attendanceLegion2)
                  return (
                  <div key={idx} className="bg-gray-700/50 rounded-lg p-4 border border-gray-600">
                    <h4 className="font-medium text-white mb-3">{legions[idx]?.name ?? `Legion ${idx + 1}`}</h4>
                    <p className="text-xs text-gray-500 mb-2">Check those who attended</p>
                    <div className="space-y-2">
                      {memberIds.map((pid) => (
                        <label
                          key={pid}
                          className="flex items-center gap-2 py-2 px-3 bg-gray-600/50 rounded cursor-pointer hover:bg-gray-600 transition-colors"
                        >
                          <input
                            type="checkbox"
                            checked={idx === 0 ? attendanceLegion1[pid] : attendanceLegion2[pid]}
                            onChange={() => toggleAttendance(idx, pid)}
                            className="w-4 h-4 rounded border-gray-500 text-green-600 focus:ring-green-500"
                          />
                          <span
                            className={
                              (idx === 0 ? attendanceLegion1[pid] : attendanceLegion2[pid])
                                ? 'text-green-400'
                                : 'text-gray-400'
                            }
                          >
                            {playerName(pid)}
                          </span>
                        </label>
                      ))}
                      {memberIds.length === 0 && (
                        <p className="text-gray-500 text-sm">No members assigned</p>
                      )}
                    </div>
                  </div>
                )})}
              </div>

              <div className="flex gap-3 justify-end">
                <button
                  onClick={() => { setShowAttendanceModal(false); setEditingRecordId(null) }}
                  className="px-4 py-2 bg-gray-600 hover:bg-gray-500 text-white rounded-lg"
                >
                  Cancel
                </button>
                <button
                  onClick={submitAttendance}
                  disabled={submittingAttendance}
                  className="px-4 py-2 bg-indigo-600 hover:bg-indigo-500 disabled:opacity-50 text-white rounded-lg font-medium"
                >
                  {submittingAttendance ? <i className="fas fa-spinner fa-spin mr-2"></i> : null}
                  Save
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}
