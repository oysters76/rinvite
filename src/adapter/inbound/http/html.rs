use crate::domain::guest::RsvpStatus;
use crate::domain::port::inbound::InviteView;

/// Minimal HTML-escape for text interpolated into the page. Guest and couple
/// names are user-controlled, so this is the guard against stored XSS.
fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

/// Render the full e-invite web page (HTML + inline JS) for a guest.
pub fn render_invite_page(view: &InviteView, token: &str) -> String {
    let e = &view.event;
    let couple = format!(
        "{} {} &amp; {} {}",
        escape(&e.bride_name),
        escape(&e.bride_family_name),
        escape(&e.groom_name),
        escape(&e.groom_family_name),
    );
    let date = e.event_date.format("%A, %e %B %Y").to_string();
    let time = format!(
        "{} – {}",
        e.start_time.format("%H:%M"),
        e.end_time.format("%H:%M")
    );
    let rsvp_by = e.rsvp_by.format("%e %B %Y").to_string();

    // The RSVP area: a read-only notice when closed, otherwise the form.
    let rsvp_section = if view.rsvp_closed {
        "<p class=\"closed\">RSVP is now closed. Please contact the hosts directly.</p>".to_owned()
    } else {
        let options: String = (1..=view.max_party_size)
            .map(|n| {
                let selected = if view.party_size == Some(n) {
                    " selected"
                } else {
                    ""
                };
                format!("<option value=\"{n}\"{selected}>{n}</option>")
            })
            .collect();

        let already = match view.rsvp_status {
            RsvpStatus::Attending => {
                "<p class=\"status\">You're currently marked as <b>attending</b>. You can update your response below.</p>"
            }
            RsvpStatus::Declined => {
                "<p class=\"status\">You're currently marked as <b>not attending</b>. You can change your response below.</p>"
            }
            RsvpStatus::Pending => "",
        };

        format!(
            r#"{already}
    <form id="rsvp">
      <fieldset>
        <legend>Will you attend?</legend>
        <label><input type="radio" name="attending" value="yes" checked> Joyfully accept</label>
        <label><input type="radio" name="attending" value="no"> Regretfully decline</label>
      </fieldset>
      <label id="party">Number attending (including you):
        <select name="party_size">{options}</select>
      </label>
      <button type="submit">Send RSVP</button>
      <p id="result" role="status"></p>
    </form>
    <script>
      const form = document.getElementById('rsvp');
      const party = document.getElementById('party');
      const result = document.getElementById('result');
      function syncParty() {{
        const attending = form.attending.value === 'yes';
        party.style.display = attending ? '' : 'none';
      }}
      form.addEventListener('change', syncParty); syncParty();
      form.addEventListener('submit', async (ev) => {{
        ev.preventDefault();
        const attending = form.attending.value === 'yes';
        const body = {{ attending, party_size: attending ? Number(form.party_size.value) : 0 }};
        result.textContent = 'Sending…';
        try {{
          const res = await fetch('/invite/{token}/rsvp', {{
            method: 'POST',
            headers: {{ 'content-type': 'application/json' }},
            body: JSON.stringify(body),
          }});
          const data = await res.json().catch(() => ({{}}));
          result.textContent = res.ok
            ? 'Thank you — your RSVP has been recorded.'
            : ('Something went wrong: ' + (data.error || res.status));
        }} catch (err) {{
          result.textContent = 'Network error, please try again.';
        }}
      }});
    </script>"#
        )
    };

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Wedding Invitation</title>
  <style>
    body {{ font-family: Georgia, 'Times New Roman', serif; margin: 0; background: #faf7f2; color: #3a3a3a; }}
    .card {{ max-width: 640px; margin: 2rem auto; background: #fff; padding: 2.5rem; border: 1px solid #e7ddd0; border-radius: 8px; box-shadow: 0 4px 24px rgba(0,0,0,.05); }}
    h1 {{ text-align: center; font-weight: normal; font-size: 2rem; margin: 0 0 .25rem; }}
    .greeting {{ text-align: center; color: #8a7a63; margin-bottom: 2rem; }}
    dl {{ display: grid; grid-template-columns: max-content 1fr; gap: .5rem 1rem; margin: 0 0 2rem; }}
    dt {{ color: #8a7a63; }}
    fieldset {{ border: 1px solid #e7ddd0; border-radius: 6px; margin: 0 0 1rem; }}
    label {{ display: block; margin: .5rem 0; }}
    button {{ font: inherit; padding: .6rem 1.4rem; background: #8a7a63; color: #fff; border: 0; border-radius: 6px; cursor: pointer; }}
    .closed, .status {{ background: #f3ede3; padding: 1rem; border-radius: 6px; }}
    #result {{ min-height: 1.2em; color: #5a5a5a; }}
  </style>
</head>
<body>
  <main class="card">
    <h1>{couple}</h1>
    <p class="greeting">Dear {guest}, you are warmly invited to our wedding.</p>
    <dl>
      <dt>Date</dt><dd>{date}</dd>
      <dt>Time</dt><dd>{time}</dd>
      <dt>Hall</dt><dd>{hall}</dd>
      <dt>Venue</dt><dd>{venue}</dd>
      <dt>RSVP by</dt><dd>{rsvp_by}</dd>
    </dl>
    {rsvp_section}
  </main>
</body>
</html>"#,
        guest = escape(&view.guest_name),
        hall = escape(&e.hall_name),
        venue = escape(&e.venue_name),
    )
}
