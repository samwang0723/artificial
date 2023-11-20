/* global $ */
/* global Prism */

let activeDiv = null;
let currentMsg = '';
let refreshBottom = true;

$(document).ready(function () {
  var origin = window.location.origin;
  var uri = origin + '/api/v1/sse';
  var sse = new EventSource(uri);
  var user_uuid;

  sse.onopen = function () {
    console.log('Connected to the server.');

    activeDiv = null;
    currentMsg = '';
    stopLoading();
  };

  sse.onerror = function () {
    console.log('Error connecting to the server.');
    stopLoading();
  };

  sse.addEventListener('user', function (msg) {
    var obj = JSON.parse(msg.data);
    var data = obj.message;

    // If the message is "[[stop]]", reset the activeDiv
    if (data === '[[stop]]') {
      formatMessage(currentMsg, true);

      activeDiv = null;
      currentMsg = '';
      stopLoading();
      return;
    } else if (data === '') {
      return;
    }

    currentMsg += data;
    if (!activeDiv) {
      addMessageRow('allison');
    }
    formatMessage(currentMsg, false);
  });

  sse.addEventListener('system', function (msg) {
    console.log('system: ' + msg.data);
    user_uuid = msg.data;
  });

  $('#chat_form').on('submit', function (e) {
    startLoading();

    e.preventDefault();
    var message = $('#message-textfield').val();
    if (message === '') {
      return;
    }

    addMessageRow('user');
    formatMessage(message, true);

    var xhr = new XMLHttpRequest();
    xhr.open('POST', origin + '/api/v1/send', true);
    xhr.setRequestHeader('Content-Type', 'application/json; charset=UTF-8');
    var data = {
      uuid: user_uuid,
      message: message
    };
    var jsonStr = JSON.stringify(data);
    xhr.send(jsonStr);

    // reset the input field and cache values
    $('#message-textfield').val('');
    $('#message-textfield').height(40);
    activeDiv = null;
    currentMsg = '';
  });

  const messageInput = document.getElementById('message-textfield');
  messageInput.addEventListener('keydown', function (event) {
    if (event.key === 'Enter' && event.shiftKey) {
      event.preventDefault();
      const value = this.value;
      this.value = value + '\n';
    }
  });
  messageInput.oninput = function () {
    messageInput.style.height = '52px';
    messageInput.style.height = Math.min(messageInput.scrollHeight, 400) + 'px';
  };

  const messages = document.getElementById('messages');
  messages.addEventListener('scroll', function () {
    // Check if the user just scrolled
    if (
      messages.scrollTop + messages.clientHeight >=
      messages.scrollHeight - 60
    ) {
      // User scrolled to the bottom, do something
      refreshBottom = true;
    } else {
      // User scrolled, but not to the bottom, do something else
      refreshBottom = false;
    }
  });
});

function startLoading() {
  document.getElementById('button-submit').style.display = 'none';
  document.getElementById('loading').style.display = 'block';
}

function stopLoading() {
  document.getElementById('button-submit').style.display = 'block';
  document.getElementById('loading').style.display = 'none';
}

function linkify(inputText) {
  var replacedText, replacePattern1, replacePattern2, replacePattern3;

  //URLs starting with http://, https://, or ftp://
  replacePattern1 =
    /(\b(https?|ftp):\/\/[-A-Z0-9+&@#/%?=~_|!:,.;]*[-A-Z0-9+&@#/%=~_|])/gim;
  replacedText = inputText.replace(replacePattern1, '[$1]($1)');

  //URLs starting with "www." (without // before it, or it'd re-link the ones done above).
  replacePattern2 = /(^|[^/])(www\.[\S]+(\b|$))/gim;
  replacedText = replacedText.replace(replacePattern2, '[$1]($2)');

  //Change email addresses to mailto:: links.
  replacePattern3 = /(([a-zA-Z0-9\-_.])+@[a-zA-Z_]+?(\.[a-zA-Z]{2,6})+)/gim;
  replacedText = replacedText.replace(replacePattern3, '[$1](mailto:$1)');

  return replacedText;
}

function boldify(inputText) {
  var replacedText, replacePattern1;

  replacePattern1 =
    /(Subject:|Summary:|Description:|Sources:|Attachments:|Similarity:|Prompt:)/gim;
  replacedText = inputText.replace(replacePattern1, '___$1___');

  return replacedText;
}

function addMessageRow(sender) {
  let messageRow = document.createElement('div');
  messageRow.classList.add('message-row');

  let messageSender = document.createElement('span');
  messageSender.classList.add('message-sender');
  messageSender.innerHTML =
    '<img width="50px" height="50px" src="https://cdn.jsdelivr.net/gh/samwang0723/project-allison@main/project_allison/static/' +
    sender +
    '.svg">';
  messageRow.appendChild(messageSender);

  let messageText = document.createElement('span');
  messageText.classList.add('message-body');
  activeDiv = messageText;
  messageRow.appendChild(messageText);

  let messageTail = document.createElement('span');
  messageTail.classList.add('message-tail');
  messageRow.appendChild(messageTail);

  let messages = document.getElementById('messages');
  messages.appendChild(messageRow);
}

function extractImageUrls(text) {
  const matches = text.match(
    /href=["'][^"']*?\.(png|jpe?g|gif|pdf|asp)(?:\?[^"']*)?["']/g
  );
  if (!matches) {
    return [];
  }
  const urls = matches.map((match) => match.slice(6, -1));
  return urls;
}

function formatMessage(message, showImg) {
  const lines = message.split('```');
  let output = '';

  for (let i = 0; i < lines.length; i++) {
    var msg = lines[i];
    if (i % 2 === 1) {
      let code_lines = msg.split('\n');
      let language = code_lines.shift().trim(); // Remove the first line, which contains the language identifier.
      let code = code_lines.join('\n');
      if (
        language === '' ||
        language === 'html' ||
        language === 'rust' ||
        language === 'markdown'
      ) {
        code = code.replace(/</g, '&lt;').replace(/>/g, '&gt;');
      }

      var code_class = 'language-';
      if (language != '') {
        code_class = 'language-' + language;
      }
      output +=
        '<pre class="prettyprint line-numbers language-markup">' +
        '<code class="' +
        code_class +
        '">' +
        code +
        '</code>' +
        '</pre>';
    } else {
      var linkified = linkify(msg);
      var boldified = boldify(linkified);
      const md = window.markdownit();
      const outputText = md.render(boldified);
      output += outputText;
    }
  }

  var images = [];
  if (showImg) {
    images = extractImageUrls(output);
    if (images.length > 0) {
      //max = images.length > 10 ? 10 : images.length;
      var imageBlock = '<div class=\'thumbnails\'>';
      for (let i = 0; i < images.length; i++) {
        const image = images[i];
        if (image.includes('.pdf')) {
          imageBlock +=
            '<div class=\'thumbnail\' data-src=\'' +
            image +
            '\' style=\'background-image:url(https://cdn.jsdelivr.net/gh/samwang0723/project-allison@main/project_allison/static/pdf.png)\'></div>';
        } else {
          imageBlock +=
            '<div class=\'thumbnail\' data-src=\'' +
            image +
            '\' style=\'background-image:url(' +
            image +
            ')\'></div>';
        }
      }
      imageBlock += '</div>';
      output += imageBlock;
    }
  }

  activeDiv.innerHTML = removeAttachments(output);
  // Apply Prism.js syntax highlighting to the newly added code block(s).
  Prism.highlightAllUnder(activeDiv);

  if (refreshBottom) {
    let messages = document.getElementById('messages');
    messages.scrollTop = messages.scrollHeight;
  }

  if (showImg && images.length > 0) {
    var elements = document.getElementsByClassName('thumbnail');
    var imageClick = function () {
      const link = this.getAttribute('data-src');
      window.open(link, '_blank');
    };
    for (var i = 0; i < elements.length; i++) {
      elements[i].addEventListener('click', imageClick, false);
    }
  }
}

function removeAttachments(html) {
  const start = html.indexOf('<em><strong>Attachments:</strong></em>');
  const end = html.indexOf('<div class=\'thumbnails\'>', start);
  if (start !== -1 && end !== -1) {
    return html.slice(0, start) + html.slice(end);
  }
  return html;
}
