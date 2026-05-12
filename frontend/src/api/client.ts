export async function apiFetch<T>(
  url: string,
  options?: RequestInit
): Promise<{ data?: T; error?: string; ok: boolean }> {
  try {
    const res = await fetch(url, {
      ...options,
      headers: {
        'Content-Type': 'application/json',
        ...options?.headers,
      },
      credentials: 'include',
    })
    const raw = await res.text()
    let parsed: unknown = {}
    if (raw) {
      try {
        parsed = JSON.parse(raw)
      } catch {
        parsed = {}
      }
    }
    const obj = parsed as { error?: string }
    if (!res.ok) {
      const fallback =
        typeof obj.error === 'string'
          ? obj.error
          : raw.trim() || `Request failed (${res.status})`
      return { ok: false, error: fallback }
    }
    return { ok: true, data: parsed as T }
  } catch (err) {
    return { ok: false, error: err instanceof Error ? err.message : 'Network error' }
  }
}

export const api = {
  login: (accountName: string, password: string) =>
    apiFetch<{ success?: boolean; account_name?: string; server_number?: number }>('/api/login', {
      method: 'POST',
      body: JSON.stringify({ account_name: accountName, password }),
    }),

  createAccount: (data: {
    account_name: string
    server_number: number
    in_game_name: string
    password: string
    player_id?: string
  }) =>
    apiFetch<{ success: boolean; schedule_url?: string }>('/api/create-account', {
      method: 'POST',
      body: JSON.stringify(data),
    }),

  getSession: () =>
    apiFetch<{
      account_name?: string
      server_number?: number
      player_id?: string
      in_game_name?: string
      is_admin?: boolean
      alliance_access?: boolean
      server_org_access?: boolean
      friend_code?: string
    }>('/api/session'),

  listAdminAccounts: () =>
    apiFetch<{ success?: boolean; accounts?: Array<{ account_name: string; server_number: number; in_game_name: string; admin: boolean }> }>(
      '/api/admin/accounts'
    ),

  setAdmin: (accountName: string, admin: boolean) =>
    apiFetch<{ success?: boolean; account_name?: string; admin?: boolean }>(
      `/api/admin/accounts/${encodeURIComponent(accountName)}/admin`,
      { method: 'POST', body: JSON.stringify({ admin }) }
    ),

  updateProfile: (data: {
    account_name?: string
    server_number?: number
    in_game_name?: string
  }) =>
    apiFetch<{ success?: boolean; account_name?: string; server_number?: number; in_game_name?: string }>(
      '/api/profile/update',
      { method: 'PUT', body: JSON.stringify(data) }
    ),

  kingshotLookup: (playerId: string) =>
    apiFetch<{
      success?: boolean
      in_game_name?: string
      server_number?: number
      player_id?: string
    }>('/api/profile/kingshot-lookup', {
      method: 'POST',
      body: JSON.stringify({ player_id: playerId }),
    }),


  logout: () => apiFetch('/api/logout', { method: 'POST' }),

  getServers: () => apiFetch<{ servers: Array<{ account_name: string; server_number: number }> }>('/api/servers'),

  getFormStats: (formCode: string) =>
    apiFetch<FormStats>(`/form/${formCode}/api/stats`),

  getFormConfig: (formCode: string) =>
    apiFetch<FormConfig>(`/form/${formCode}/api/config`),

  submitForm: (formCode: string, data: FormSubmission) =>
    apiFetch<{ success: boolean; error?: string }>(`/form/${formCode}/api/submit`, {
      method: 'POST',
      body: JSON.stringify(data),
    }),

  getSchedule: (account: string, server: number, day: string) =>
    apiFetch<Schedule>(`/${account}/${server}/api/schedule/${day}`),

  getScheduleByFormCode: (account: string, formCode: string, day: string) =>
    apiFetch<Schedule>(`/api/public-schedule/${encodeURIComponent(account)}/${encodeURIComponent(formCode)}/${day}`),

  updateScheduleSlot: (account: string, server: number, day: string, time: string, player: string) =>
    apiFetch<{ success?: boolean }>(`/${account}/${server}/api/schedule/${day}/slot`, {
      method: 'PUT',
      body: JSON.stringify({ time, player }),
    }),

  getStats: (account: string, server: number) =>
    apiFetch<AccountStats>(`/${account}/${server}/api/stats`),

  accountLogin: (account: string, server: number, password: string) =>
    apiFetch<{ success: boolean }>(`/${account}/${server}/api/login`, {
      method: 'POST',
      body: JSON.stringify({ password }),
    }),

  uploadCsv: (account: string, server: number, file: File) => {
    const formData = new FormData()
    formData.append('file', file)
    return fetch(`/${account}/${server}/api/upload`, {
      method: 'POST',
      body: formData,
      credentials: 'include',
    }).then(async (res) => {
      const data = await res.json().catch(() => ({}))
      return { ok: res.ok, data, error: (data as { error?: string }).error }
    })
  },

  createForm: (account: string, server: number, config: CreateFormRequest) =>
    apiFetch<{ success: boolean; url?: string; form_url?: string; code?: string; form_code?: string }>(
      `/${account}/${server}/api/form/create`,
      { method: 'POST', body: JSON.stringify(config) }
    ),

  getCurrentForm: (account: string, server: number) =>
    apiFetch<CurrentFormInfo>(`/${account}/${server}/api/form/current`),

  listOldForms: (account: string, server: number) =>
    apiFetch<{ old_forms?: Array<{ archive_name: string; code: string; name: string; created_at: string; delete_date?: string }> }>(
      `/${account}/${server}/api/form/old`
    ),

  reopenForm: (account: string, server: number, archiveName: string) =>
    apiFetch<{ success?: boolean; code?: string; url?: string }>(`/${account}/${server}/api/form/reopen`, {
      method: 'POST',
      body: JSON.stringify({ archive_name: archiveName }),
    }),

  clearSchedule: (account: string, server: number, day?: 'construction' | 'research' | 'troops') =>
    apiFetch<{ success?: boolean; message?: string }>(`/${account}/${server}/api/schedule/clear`, {
      method: 'POST',
      body: JSON.stringify(day ? { day } : {}),
    }),

  getFormSubmissions: (account: string, server: number) =>
    apiFetch<{ submissions?: Record<string, unknown>[] }>(`/${account}/${server}/api/form/submissions`),

  listAlliances: (account: string, server: number) =>
    apiFetch<{
      alliances?: Array<{
        name: string
        slug: string
        players: AlliancePlayer[]
        owner_account: string
        owner_server: number
        is_owner: boolean
      }>
    }>(`/${account}/${server}/api/alliances`),

  getFriendCode: (account: string, server: number) =>
    apiFetch<{ success?: boolean; friend_code?: string }>(`/${account}/${server}/api/friend-code`),

  listAllianceInvites: (account: string, server: number) =>
    apiFetch<{
      success?: boolean
      sent?: Array<{ id: string; type: string; to_friend_code: string; to_account: string; alliance_name: string; status: string; created_at: string }>
      received?: Array<{ id: string; type: string; from_account: string; alliance_name: string; status: string; created_at: string }>
    }>(`/${account}/${server}/api/alliance-invites`),

  createAllianceInvite: (account: string, server: number, friendCode: string) =>
    apiFetch<{ success?: boolean; error?: string }>(`/${account}/${server}/api/alliance-invites`, {
      method: 'POST',
      body: JSON.stringify({ friend_code: friendCode }),
    }),

  acceptAllianceInvite: (account: string, server: number, inviteId: string) =>
    apiFetch<{ success?: boolean; error?: string }>(`/${account}/${server}/api/alliance-invites/${inviteId}/accept`, {
      method: 'POST',
    }),

  rejectAllianceInvite: (account: string, server: number, inviteId: string) =>
    apiFetch<{ success?: boolean; error?: string }>(`/${account}/${server}/api/alliance-invites/${inviteId}/reject`, {
      method: 'POST',
    }),

  revokeAllianceInvite: (account: string, server: number, inviteId: string) =>
    apiFetch<{ success?: boolean; error?: string }>(`/${account}/${server}/api/alliance-invites/${inviteId}/revoke`, {
      method: 'POST',
    }),

  addAllianceMember: (account: string, server: number, allianceName: string, playerId: string) =>
    apiFetch<{ success?: boolean; player?: AlliancePlayer }>(`/${account}/${server}/api/alliance-members`, {
      method: 'POST',
      body: JSON.stringify({ alliance_name: allianceName, player_id: playerId }),
    }),

  removeAllianceMember: (account: string, server: number, allianceSlug: string, playerId: string) =>
    apiFetch<{ success?: boolean }>(
      `/${account}/${server}/api/alliance-members/${encodeURIComponent(allianceSlug)}/${encodeURIComponent(playerId)}`,
      { method: 'DELETE' }
    ),

  refreshAllianceNames: (account: string, server: number, allianceSlug: string) =>
    apiFetch<{ success?: boolean; updated?: number; total?: number; error?: string }>(
      `/${account}/${server}/api/alliances/${encodeURIComponent(allianceSlug)}/refresh-names`,
      { method: 'POST' }
    ),

  getSwordland: (ownerAccount: string, ownerServer: number, allianceSlug: string) =>
    apiFetch<{
      success?: boolean
      legions?: Array<{ name: string; member_ids: string[]; filler_ids?: string[] }>
      attendance_records?: Array<{
        id: string
        date: string
        label?: string
        legion_1: { attended: string[]; absent: string[]; filler?: string[] }
        legion_2: { attended: string[]; absent: string[]; filler?: string[] }
      }>
    }>(`/${ownerAccount}/${ownerServer}/api/alliances/${encodeURIComponent(allianceSlug)}/swordland`),

  setSwordlandLegions: (
    ownerAccount: string,
    ownerServer: number,
    allianceSlug: string,
    legions: Array<{ name: string; member_ids: string[]; filler_ids?: string[] }>
  ) =>
    apiFetch<{ success?: boolean; legions?: Array<{ name: string; member_ids: string[] }> }>(
      `/${ownerAccount}/${ownerServer}/api/alliances/${encodeURIComponent(allianceSlug)}/swordland`,
      {
        method: 'PUT',
        body: JSON.stringify({ legions }),
      }
    ),

  addSwordlandAttendance: (
    ownerAccount: string,
    ownerServer: number,
    allianceSlug: string,
    data: {
      date: string
      label?: string
      legion_1_attended: string[]
      legion_1_absent: string[]
      legion_1_filler?: string[]
      legion_2_attended: string[]
      legion_2_absent: string[]
      legion_2_filler?: string[]
    }
  ) =>
    apiFetch<{
      success?: boolean
      record?: {
        id: string
        date: string
        label?: string
        legion_1: { attended: string[]; absent: string[] }
        legion_2: { attended: string[]; absent: string[] }
      }
    }>(`/${ownerAccount}/${ownerServer}/api/alliances/${encodeURIComponent(allianceSlug)}/swordland/attendance`, {
      method: 'POST',
      body: JSON.stringify(data),
    }),

  updateSwordlandAttendance: (
    ownerAccount: string,
    ownerServer: number,
    allianceSlug: string,
    recordId: string,
    data: {
      date: string
      label?: string
      legion_1_attended: string[]
      legion_1_absent: string[]
      legion_1_filler?: string[]
      legion_2_attended: string[]
      legion_2_absent: string[]
      legion_2_filler?: string[]
    }
  ) =>
    apiFetch<{
      success?: boolean
      record?: {
        id: string
        date: string
        label?: string
        legion_1: { attended: string[]; absent: string[] }
        legion_2: { attended: string[]; absent: string[] }
      }
    }>(`/${ownerAccount}/${ownerServer}/api/alliances/${encodeURIComponent(allianceSlug)}/swordland/attendance/${recordId}`, {
      method: 'PUT',
      body: JSON.stringify(data),
    }),

  getTriAlliance: (ownerAccount: string, ownerServer: number, allianceSlug: string) =>
    apiFetch<{
      success?: boolean
      legions?: Array<{ name: string; member_ids: string[]; filler_ids?: string[] }>
      attendance_records?: Array<{
        id: string
        date: string
        label?: string
        legion_1: { attended: string[]; absent: string[]; filler?: string[] }
        legion_2: { attended: string[]; absent: string[]; filler?: string[] }
      }>
    }>(`/${ownerAccount}/${ownerServer}/api/alliances/${encodeURIComponent(allianceSlug)}/tri-alliance`),

  setTriAllianceLegions: (
    ownerAccount: string,
    ownerServer: number,
    allianceSlug: string,
    legions: Array<{ name: string; member_ids: string[]; filler_ids?: string[] }>
  ) =>
    apiFetch<{ success?: boolean; legions?: Array<{ name: string; member_ids: string[] }> }>(
      `/${ownerAccount}/${ownerServer}/api/alliances/${encodeURIComponent(allianceSlug)}/tri-alliance`,
      {
        method: 'PUT',
        body: JSON.stringify({ legions }),
      }
    ),

  addTriAllianceAttendance: (
    ownerAccount: string,
    ownerServer: number,
    allianceSlug: string,
    data: {
      date: string
      label?: string
      legion_1_attended: string[]
      legion_1_absent: string[]
      legion_1_filler?: string[]
      legion_2_attended: string[]
      legion_2_absent: string[]
      legion_2_filler?: string[]
    }
  ) =>
    apiFetch<{
      success?: boolean
      record?: {
        id: string
        date: string
        label?: string
        legion_1: { attended: string[]; absent: string[] }
        legion_2: { attended: string[]; absent: string[] }
      }
    }>(`/${ownerAccount}/${ownerServer}/api/alliances/${encodeURIComponent(allianceSlug)}/tri-alliance/attendance`, {
      method: 'POST',
      body: JSON.stringify(data),
    }),

  updateTriAllianceAttendance: (
    ownerAccount: string,
    ownerServer: number,
    allianceSlug: string,
    recordId: string,
    data: {
      date: string
      label?: string
      legion_1_attended: string[]
      legion_1_absent: string[]
      legion_1_filler?: string[]
      legion_2_attended: string[]
      legion_2_absent: string[]
      legion_2_filler?: string[]
    }
  ) =>
    apiFetch<{
      success?: boolean
      record?: {
        id: string
        date: string
        label?: string
        legion_1: { attended: string[]; absent: string[] }
        legion_2: { attended: string[]; absent: string[] }
      }
    }>(`/${ownerAccount}/${ownerServer}/api/alliances/${encodeURIComponent(allianceSlug)}/tri-alliance/attendance/${recordId}`, {
      method: 'PUT',
      body: JSON.stringify(data),
    }),

  getGiftcodeRecipients: (account: string, server: number) =>
    apiFetch<{ success?: boolean; player_ids?: string[] }>(`/${account}/${server}/api/giftcode-recipients`),

  setGiftcodeRecipients: (account: string, server: number, playerIds: string[]) =>
    apiFetch<{ success?: boolean; player_ids?: string[] }>(`/${account}/${server}/api/giftcode-recipients`, {
      method: 'PUT',
      body: JSON.stringify({ player_ids: playerIds }),
    }),

  redeemGiftcode: (account: string, server: number, giftcode: string) =>
    apiFetch<{
      success?: boolean
      results?: Array<{ player_id: string; status: string; message: string }>
    }>(`/${account}/${server}/api/redeem-giftcode`, {
      method: 'POST',
      body: JSON.stringify({ giftcode: giftcode.trim() }),
    }),

  fetchGiftcodes: (account: string, server: number) =>
    apiFetch<{
      success?: boolean
      codes?: Array<{ code: string; date: string }>
    }>(`/${account}/${server}/api/fetch-giftcodes`),

  downloadFormCsv: (account: string, server: number) =>
    fetch(`/${account}/${server}/api/form/download-csv`, { credentials: 'include' }),

  updateFormConfig: (account: string, server: number, predeterminedSlots: PredeterminedSlot[]) =>
    apiFetch<{ success?: boolean }>(`/${account}/${server}/api/form/config`, {
      method: 'PUT',
      body: JSON.stringify({ predetermined_slots: predeterminedSlots }),
    }),

  getPlayerById: (account: string, server: number, playerId: string) =>
    apiFetch<{ name?: string; alliance?: string }>(`/${account}/${server}/api/form/player/${playerId}`),

  getPreviousFormConfig: (account: string, server: number) =>
    apiFetch<{ success?: boolean; config?: { alliances?: string[]; include_non_of_above?: boolean; [key: string]: unknown } }>(`/${account}/${server}/api/form/previous`),

  generateSchedule: (append: boolean, day?: 'construction' | 'research' | 'troops') =>
    apiFetch<{ success?: boolean; message?: string; error?: string }>('/api/generate-schedule', {
      method: 'POST',
      body: JSON.stringify({ append, ...(day && { day }) }),
    }),

  checkSubmission: (formCode: string, playerId: string) =>
    apiFetch<{ has_submitted: boolean }>(`/form/${formCode}/api/check-submission/${playerId}`),

  playerLookup: (formCode: string, playerId: string) =>
    apiFetch<PlayerCard>(`/form/${formCode}/api/player-lookup/${playerId}`),

  submitFeedback: (data: { type: 'bug' | 'feature' | 'general'; text: string }) =>
    apiFetch<{ success?: boolean }>('/api/feedback', {
      method: 'POST',
      body: JSON.stringify(data),
    }),

  listFeedback: () =>
    apiFetch<{ success?: boolean; feedback?: Array<{ id: string; type: string; text: string; created_at: string }> }>(
      '/api/admin/feedback'
    ),

  archiveFeedback: (id: string) =>
    apiFetch<{ success?: boolean }>(`/api/admin/feedback/${encodeURIComponent(id)}/archive`, {
      method: 'POST',
    }),

  getMyAllianceApplication: () =>
    apiFetch<{ success?: boolean; application?: AllianceApplication }>('/api/alliance-application'),

  submitAllianceApplication: (data: {
    alliance_tag: string
    alliance_name: string
    contact_player_id: string
    server_number: number
  }) =>
    apiFetch<{ success?: boolean; error?: string }>('/api/alliance-application', {
      method: 'POST',
      body: JSON.stringify(data),
    }),

  listAllianceApplications: () =>
    apiFetch<{ success?: boolean; applications?: AllianceApplication[] }>('/api/admin/alliance-applications'),

  approveAllianceApplication: (id: string) =>
    apiFetch<{ success?: boolean }>(`/api/admin/alliance-applications/${encodeURIComponent(id)}/approve`, {
      method: 'POST',
    }),

  rejectAllianceApplication: (id: string) =>
    apiFetch<{ success?: boolean }>(`/api/admin/alliance-applications/${encodeURIComponent(id)}/reject`, {
      method: 'POST',
    }),

  listServerOrgWorkspaces: (account: string, server: number) =>
    apiFetch<{
      success?: boolean
      workspaces?: Array<{
        id: string
        display_name: string
        kingshot_server_number: number
        owner_account_key: string
        created_at?: string
        /** Present when a Tyrant form exists for this workspace. */
        tyrant_public_code?: string | null
      }>
    }>(`/${account}/${server}/api/server-org/workspaces`),

  createServerOrgWorkspace: (account: string, server: number, display_name: string) =>
    apiFetch<{ success?: boolean; workspace_id?: string; error?: string }>(
      `/${account}/${server}/api/server-org/workspaces`,
      { method: 'POST', body: JSON.stringify({ display_name }) }
    ),

  createServerOrgWorkspaceInvite: (account: string, server: number, workspace_id: string, friend_code: string) =>
    apiFetch<{ success?: boolean; invite_id?: string }>(
      `/${account}/${server}/api/server-org/workspaces/${encodeURIComponent(workspace_id)}/invites`,
      { method: 'POST', body: JSON.stringify({ friend_code }) }
    ),

  listServerOrgInvites: (account: string, server: number) =>
    apiFetch<{
      success?: boolean
      sent?: Array<Record<string, unknown>>
      received?: Array<Record<string, unknown>>
    }>(`/${account}/${server}/api/server-org/invites`),

  acceptServerOrgInvite: (account: string, server: number, invite_id: string) =>
    apiFetch<{ success?: boolean }>(
      `/${account}/${server}/api/server-org/invites/${encodeURIComponent(invite_id)}/accept`,
      { method: 'POST', body: JSON.stringify({}) }
    ),

  ensureTyrantForm: (
    account: string,
    server: number,
    workspace_id: string,
    config: { alliances?: string[]; include_non_of_above?: boolean; kingdom_id?: string; utc_slots_note?: string }
  ) =>
    apiFetch<{ success?: boolean; form_id?: string; public_code?: string }>(
      `/${account}/${server}/api/server-org/workspaces/${encodeURIComponent(workspace_id)}/tyrant-form`,
      { method: 'POST', body: JSON.stringify(config) }
    ),

  listTyrantSubmissions: (account: string, server: number, workspace_id: string, sort?: string) => {
    const q = sort ? `?sort=${encodeURIComponent(sort)}` : ''
    return apiFetch<{
      success?: boolean
      submissions?: Array<Record<string, unknown>>
      sort?: string
    }>(
      `/${account}/${server}/api/server-org/workspaces/${encodeURIComponent(workspace_id)}/tyrant-submissions${q}`
    )
  },

  /** Public Tyrant form (no credentials). */
  getTyrantFormConfig: (code: string) =>
    apiFetch<{ workspace_id?: string; config?: Record<string, unknown> }>(`/tyrant-form/${code}/api/config`),

  tyrantSubmit: (
    code: string,
    data: {
      player_id: string
      player_name: string
      alliance: string
      archer: { level_band: string; tg_band: string }
      cavalry: { level_band: string; tg_band: string }
      infantry: { level_band: string; tg_band: string }
      utc_slots?: string[]
      participate_full_five_hours?: boolean
      auto_help_month_card_active: boolean
    }
  ) =>
    apiFetch<{ success?: boolean }>(`/tyrant-form/${code}/api/submit`, {
      method: 'POST',
      body: JSON.stringify(data),
    }),

  tyrantPlayerLookup: (code: string, playerId: string) =>
    apiFetch<PlayerCard & { kingdom_mismatch?: boolean; success?: boolean; error?: string | unknown }>(
      `/tyrant-form/${encodeURIComponent(code)}/api/player-lookup/${encodeURIComponent(playerId)}`
    ),
}

export interface AllianceApplication {
  id: string
  account_name: string
  alliance_tag: string
  alliance_name: string
  contact_player_id: string
  server_number: number
  status: 'pending' | 'approved' | 'rejected'
  submitted_at: string
}

export interface AlliancePlayer {
  player_id: string
  name: string
  castle_level?: string
  kingdom?: string
  avatar_image?: string
  added_at: string
}

export interface AccountStats {
  alliance_counts?: Record<string, { construction_requests: number; research_requests: number; troops_requests: number }>
  time_slot_popularity?: Record<string, { construction_requests: number; research_requests: number; troops_requests: number }>
  construction_time_slot_popularity?: Record<string, { requests: number }>
  research_time_slot_popularity?: Record<string, { requests: number }>
  troops_time_slot_popularity?: Record<string, { requests: number }>
  construction_start_time?: string
  research_start_time?: string
  troops_start_time?: string
}

export interface CreateFormRequest {
  name?: string
  form_name?: string
  kingdom_id: string
  support_person_name?: string
  alliances: string[]
  include_non_of_above: boolean
  construction_truegold_mode: string
  construction_times: { start_time: string; end_time?: string }
  research_times: { start_time: string; end_time?: string }
  troops_times: { start_time: string; end_time?: string }
   // Logical day slots (e.g. 'monday', 'tuesday', 'friday_full', 'friday_sat') for display/translation
  construction_day_slot?: string
  research_day_slot?: string
  predetermined_slots?: PredeterminedSlot[]
  intro_text?: string
}

export interface PredeterminedSlot {
  day: string
  time: string
  player_id?: string
  alliance?: string
  name?: string
}

export interface CurrentFormInfo {
  code: string
  name: string
  url: string
  delete_date?: string
  created_at: string
  submissions_count?: number
}

export interface PlayerCard {
  player_id: string
  name: string
  castle_level?: number | string
  kingdom?: string
  avatar_image?: string
}

export interface FormStats {
  construction_time_slot_popularity?: Record<string, { requests: number }>
  research_time_slot_popularity?: Record<string, { requests: number }>
  troops_time_slot_popularity?: Record<string, { requests: number }>
  construction_start_time?: string
  research_start_time?: string
  troops_start_time?: string
}

export interface FormConfig {
  alliances?: string[]
  include_non_of_above?: boolean
  construction_truegold_mode?: string
  construction_times?: { start_time: string; end_time?: string | null }
  research_times?: { start_time: string; end_time?: string | null }
  troops_times?: { start_time: string; end_time?: string | null }
  construction_day_slot?: string | null
  research_day_slot?: string | null
  intro_text?: string
  support_person_name?: string
  kingdom_id?: string
  [key: string]: unknown
}

export interface FormSubmission {
  alliance: string
  custom_alliance?: string
  character_name: string
  player_id: string
  submission_type: string
  wants_construction: boolean
  construction_speedups?: number
  construction_truegold?: number
  construction_tempered_truegold?: number
  construction_time_slots: number[]
  wants_research: boolean
  research_speedups?: number
  research_truegold_dust?: number
  research_time_slots: number[]
  wants_troops: boolean
  troops_speedups?: number
  troops_time_slots: number[]
  additional_notes?: string
  suggestions?: string
}

export interface Schedule {
  day_name?: string
  appointments?: Array<{ time: string; player?: string; is_empty?: boolean }>
  [key: string]: unknown
}
