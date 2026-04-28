import { useCallback, useEffect, useMemo, useState } from 'react'
import { useAuth } from '../../context/AuthContext'
import { api } from '../../api/client'

interface TabManageServerProps {
  accountName: string | null
  serverNumber: number | null
}

type WorkspaceRow = {
  id: string
  display_name: string
  kingshot_server_number: number
  owner_account_key: string
  created_at?: string
}

export default function TabManageServer({ accountName, serverNumber }: TabManageServerProps) {
  const { refresh: refreshAuth } = useAuth()
  const [workspaces, setWorkspaces] = useState<WorkspaceRow[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  const [newName, setNewName] = useState('')
  const [creating, setCreating] = useState(false)

  const [selectedId, setSelectedId] = useState<string | null>(null)
  const selected = useMemo(
    () => workspaces.find((w) => w.id === selectedId) ?? null,
    [workspaces, selectedId]
  )

  const [allyInput, setAllyInput] = useState('')
  const [kingdomId, setKingdomId] = useState('')
  const [includeNon, setIncludeNon] = useState(true)
  const [savingForm, setSavingForm] = useState(false)

  const [inviteCode, setInviteCode] = useState('')
  const [inviteBusy, setInviteBusy] = useState(false)

  const [received, setReceived] = useState<Array<{ id?: string } & Record<string, unknown>>>([])
  const [accepting, setAccepting] = useState<string | null>(null)

  const [formMsg, setFormMsg] = useState<string | null>(null)

  const load = useCallback(async () => {
    if (!accountName || serverNumber == null) return
    setLoading(true)
    setError(null)
    const r = await api.listServerOrgWorkspaces(accountName, serverNumber)
    setLoading(false)
    if (r.ok && r.data?.success && r.data.workspaces) {
      setWorkspaces(r.data.workspaces)
      setSelectedId((prev) =>
        prev && r.data!.workspaces!.some((x) => x.id === prev) ? prev : r.data!.workspaces![0]?.id ?? null
      )
    } else {
      setError(r.error ?? 'Failed to load workspaces')
    }
  }, [accountName, serverNumber])

  const loadInvites = useCallback(async () => {
    if (!accountName || serverNumber == null) return
    const r = await api.listServerOrgInvites(accountName, serverNumber)
    if (r.ok && r.data?.success && Array.isArray(r.data.received)) {
      setReceived(r.data.received as Array<{ id?: string } & Record<string, unknown>>)
    }
  }, [accountName, serverNumber])

  useEffect(() => {
    load()
  }, [load])

  useEffect(() => {
    loadInvites()
  }, [loadInvites])

  async function handleCreate(e: React.FormEvent) {
    e.preventDefault()
    if (!accountName || serverNumber == null || !newName.trim()) return
    setCreating(true)
    const r = await api.createServerOrgWorkspace(accountName, serverNumber, newName.trim())
    setCreating(false)
    if (r.ok && r.data?.success) {
      setNewName('')
      await load()
      await refreshAuth()
    } else {
      setError(r.data?.error ?? r.error ?? 'Create failed')
    }
  }

  async function handleSaveTyrantConfig(e: React.FormEvent) {
    e.preventDefault()
    if (!accountName || serverNumber == null || !selected) return
    const alliances = allyInput
      .split(/[\n,]/)
      .map((s) => s.trim())
      .filter(Boolean)
    setSavingForm(true)
    setFormMsg(null)
    const r = await api.ensureTyrantForm(accountName, serverNumber, selected.id, {
      alliances,
      include_non_of_above: includeNon,
      kingdom_id: kingdomId.trim(),
    })
    setSavingForm(false)
    if (!r.ok || !r.data?.success) {
      setError(r.error ?? 'Failed to save Tyrant form')
      return
    }
    setFormMsg(`Public Tyrant URL: ${window.location.origin}/tyrant-form/${r.data.public_code}`)
  }

  async function handleInvite(e: React.FormEvent) {
    e.preventDefault()
    if (!accountName || serverNumber == null || !selected) return
    setInviteBusy(true)
    const r = await api.createServerOrgWorkspaceInvite(accountName, serverNumber, selected.id, inviteCode.trim())
    setInviteBusy(false)
    if (r.ok && r.data?.success) {
      setInviteCode('')
      await loadInvites()
      await refreshAuth()
    } else {
      setError(r.error ?? 'Invite failed')
    }
  }

  async function accept(id: string) {
    if (!accountName || serverNumber == null) return
    setAccepting(id)
    const r = await api.acceptServerOrgInvite(accountName, serverNumber, id)
    setAccepting(null)
    if (r.ok && r.data?.success) {
      await Promise.all([load(), loadInvites()])
      await refreshAuth()
    }
  }

  if (!accountName || serverNumber == null) {
    return <p className="text-gray-400">Missing account context.</p>
  }

  return (
    <div className="max-w-4xl mx-auto space-y-8">
      <div className="text-center mb-8">
        <div className="inline-block bg-teal-900/50 rounded-full p-4 mb-4">
          <i className="fas fa-building text-teal-400 text-3xl"></i>
        </div>
        <h2 className="text-3xl font-bold text-white mb-2">Manage server workspaces</h2>
        <p className="text-gray-400">
          Create workspaces for Kingshot server {serverNumber}; invite co-admins by friend code (12 characters).
        </p>
      </div>

      {loading && (
        <div className="text-center text-gray-400">
          <i className="fas fa-spinner fa-spin mr-2"></i>
          Loading...
        </div>
      )}
      {error && (
        <div className="bg-red-900/50 border border-red-600/60 text-red-200 px-4 py-3 rounded-lg">{error}</div>
      )}

      <section className="bg-gray-800 rounded-xl border border-gray-700 p-6">
        <h3 className="text-lg font-semibold text-white mb-4">
          <i className="fas fa-plus-circle text-teal-400 mr-2"></i>
          Create workspace
        </h3>
        <form onSubmit={handleCreate} className="flex flex-wrap gap-2 items-end">
          <div className="flex-1 min-w-[200px]">
            <label className="block text-xs text-gray-400 mb-1">Display name</label>
            <input
              className="w-full px-3 py-2 rounded-lg bg-gray-900 border border-gray-600 text-white"
              value={newName}
              onChange={(e) => setNewName(e.target.value)}
              placeholder="e.g. State 123 Tyrant coordination"
            />
          </div>
          <button
            type="submit"
            disabled={creating || !newName.trim()}
            className="px-4 py-2 rounded-lg bg-teal-600 hover:bg-teal-500 text-white font-medium disabled:opacity-50"
          >
            {creating ? 'Creating…' : 'Create'}
          </button>
        </form>
      </section>

      {workspaces.length > 0 && (
        <>
          <section className="bg-gray-800 rounded-xl border border-gray-700 p-6">
            <h3 className="text-lg font-semibold text-white mb-4">Your workspaces</h3>
            <div className="flex flex-wrap gap-2 mb-4">
              {workspaces.map((w) => (
                <button
                  key={w.id}
                  type="button"
                  onClick={() => setSelectedId(w.id)}
                  className={`px-4 py-2 rounded-lg border text-sm ${
                    selectedId === w.id
                      ? 'bg-teal-600/90 border-teal-500 text-white'
                      : 'bg-gray-900 border-gray-600 text-gray-300 hover:border-gray-500'
                  }`}
                >
                  {w.display_name}
                </button>
              ))}
            </div>
            {selected && (
              <div className="text-sm text-gray-400 space-y-1 border-t border-gray-700 pt-4">
                <p>
                  <span className="text-gray-500">Workspace ID:</span> <code className="text-gray-300">{selected.id}</code>
                </p>
                <p>
                  <span className="text-gray-500">Kingshot server #:</span> {selected.kingshot_server_number}
                </p>
              </div>
            )}
          </section>

          {selected && (
            <>
              <section className="bg-gray-800 rounded-xl border border-gray-700 p-6">
                <h3 className="text-lg font-semibold text-white mb-4">
                  <i className="fas fa-user-plus text-teal-400 mr-2"></i>
                  Invite co-admin
                </h3>
                <form onSubmit={handleInvite} className="flex flex-wrap gap-2 items-end">
                  <div className="flex-1 min-w-[200px]">
                    <label className="block text-xs text-gray-400 mb-1">12-character friend code</label>
                    <input
                      className="w-full px-3 py-2 rounded-lg bg-gray-900 border border-gray-600 text-white font-mono"
                      value={inviteCode}
                      onChange={(e) => setInviteCode(e.target.value.replace(/\s/g, ''))}
                      maxLength={12}
                      autoCapitalize="off"
                    />
                  </div>
                  <button
                    type="submit"
                    disabled={inviteBusy || inviteCode.trim().length !== 12}
                    className="px-4 py-2 rounded-lg bg-indigo-600 hover:bg-indigo-500 text-white font-medium disabled:opacity-50"
                  >
                    {inviteBusy ? 'Sending…' : 'Send invite'}
                  </button>
                </form>
              </section>

              <section className="bg-gray-800 rounded-xl border border-gray-700 p-6">
                <h3 className="text-lg font-semibold text-white mb-4">
                  <i className="fas fa-dragon text-cyan-400 mr-2"></i>
                  Tyrant public form (share link)
                </h3>
                <p className="text-gray-400 text-sm mb-4">
                  Save configuration, then players use <code className="text-gray-300">/tyrant-form/&lt;code&gt;</code>. Paste
                  alliance tags one per line or comma-separated; optional kingdom id for lookup checks.
                </p>
                {formMsg && (
                  <div className="bg-teal-900/40 border border-teal-600/60 text-teal-100 px-4 py-3 rounded-lg text-sm break-all mb-4">
                    {formMsg}
                  </div>
                )}
                <form onSubmit={handleSaveTyrantConfig} className="space-y-4">
                  <div>
                    <label className="block text-xs text-gray-400 mb-1">Alliance tags (whitelist)</label>
                    <textarea
                      className="w-full px-3 py-2 rounded-lg bg-gray-900 border border-gray-600 text-white min-h-[100px]"
                      value={allyInput}
                      onChange={(e) => setAllyInput(e.target.value)}
                      placeholder={'[LEG1]\n[LEG2]'}
                    />
                  </div>
                  <div className="flex flex-wrap gap-4 items-center">
                    <label className="flex items-center gap-2 text-gray-300 cursor-pointer">
                      <input type="checkbox" checked={includeNon} onChange={(e) => setIncludeNon(e.target.checked)} />
                      Include &quot;Non of the above&quot;
                    </label>
                  </div>
                  <div>
                    <label className="block text-xs text-gray-400 mb-1">
                      Kingdom id (optional, for player lookup gate)
                    </label>
                    <input
                      className="w-full max-w-xs px-3 py-2 rounded-lg bg-gray-900 border border-gray-600 text-white"
                      value={kingdomId}
                      onChange={(e) => setKingdomId(e.target.value)}
                      placeholder=""
                    />
                  </div>
                  <button
                    type="submit"
                    disabled={savingForm}
                    className="px-4 py-2 rounded-lg bg-cyan-600 hover:bg-cyan-500 text-white font-medium disabled:opacity-50"
                  >
                    {savingForm ? 'Saving…' : 'Save Tyrant form & public code'}
                  </button>
                </form>
              </section>
            </>
          )}
        </>
      )}

      {received.length > 0 && (
        <section className="bg-gray-800 rounded-xl border border-gray-700 p-6">
          <h3 className="text-lg font-semibold text-white mb-4">Received invites</h3>
          <ul className="space-y-2">
            {received.map((inv) => {
              const rid = typeof inv.id === 'string' ? inv.id : ''
              return (
                <li key={rid} className="flex items-center justify-between gap-4 bg-gray-900/70 rounded-lg px-4 py-3">
                  <span className="text-gray-300">
                    Invite to workspace{' '}
                    <code className="text-teal-300">{String(inv.workspace_id ?? '')}</code>{' '}
                    from{' '}
                    <strong>{String(inv.from_account ?? '')}</strong>
                  </span>
                  {rid ? (
                    <button
                      type="button"
                      disabled={accepting === rid}
                      onClick={() => accept(rid)}
                      className="px-3 py-1.5 rounded-lg bg-teal-600 hover:bg-teal-500 text-white text-sm"
                    >
                      {accepting === rid ? '…' : 'Accept'}
                    </button>
                  ) : null}
                </li>
              )
            })}
          </ul>
        </section>
      )}
    </div>
  )
}
