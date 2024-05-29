/* global $ */
/* global Prism */

let activeDiv = null;
let currentMsg = '';
let currentImage = '';
let refreshBottom = true;
let currentVendor = 'openai';
let hasIndexDB = false;

$(document).ready(function() {

  if (!window.indexedDB) {
    console.log('Your browser does not support a stable version of IndexedDB. Some features will not be available.');
  } else {
    console.log('IndexedDB is supported.');

    const request = indexedDB.open('artifical-chat', 1);

    request.onupgradeneeded = function(event) {
      const db = event.target.result;
      if (!db.objectStoreNames.contains('messages')) {
        db.createObjectStore('messages', { keyPath: 'id', autoIncrement: true });
      }
    };

    request.onerror = function(event) {
      console.error('Database error: ', event.target.errorCode);
    };

    request.onsuccess = function(event) {
      hasIndexDB = true;
      console.log('Database opened successfully');

      loadMessages(displayHistoricalMessages);
    };
  }

  var origin = window.location.origin;
  var uri = origin + '/api/v1/sse';
  var sse = new EventSource(uri);
  var user_uuid;

  sse.onopen = function() {
    console.log('Connected to the server.');

    activeDiv = null;
    currentMsg = '';
    stopLoading();
  };

  sse.onerror = function() {
    console.log('Error connecting to the server.');
    stopLoading();
  };

  sse.addEventListener('user', function(msg) {
    var obj = JSON.parse(msg.data);
    var data = obj.message;

    // If the message is "[[stop]]", reset the activeDiv
    if (data === '[[stop]]') {
      if (currentMsg !== '') {
        formatMessage(currentMsg, true);
      }
      storeMessage('allison', currentMsg);

      activeDiv = null;
      currentMsg = '';
      stopLoading();
      return;
    }

    currentMsg += data;
    if (!activeDiv) {
      addMessageRow('allison');
    }
    formatMessage(currentMsg, false);
  });

  sse.addEventListener('system', function(msg) {
    user_uuid = msg.data;
  });

  $('#chat_form').on('submit', function(e) {
    startLoading();

    e.preventDefault();
    var message = $('#message-textfield').val();
    if (message === '') {
      return;
    }

    addMessageRow('user');
    formatMessage(message + '\n' + currentImage, true);

    var xhr = new XMLHttpRequest();
    xhr.open('POST', origin + '/api/v1/send/' + currentVendor, true);
    xhr.setRequestHeader('Content-Type', 'application/json; charset=UTF-8');
    var data = {
      uuid: user_uuid,
      message: message
    };

    let storedMessage = message;

    // append image url if exists
    if (currentImage !== '') {
      data['image'] = currentImage;
      removeImage();

      storedMessage += '\n' + currentImage;
    }
    storeMessage('user', storedMessage);

    var jsonStr = JSON.stringify(data);
    xhr.send(jsonStr);

    // reset the input field and cache values
    $('#message-textfield').val('');
    $('#message-textfield').height(26);
    activeDiv = null;
    currentMsg = '';
  });

  const messageInput = document.getElementById('message-textfield');
  messageInput.addEventListener('keydown', function(event) {
    if (event.key === 'Enter' && event.shiftKey) {
      event.preventDefault();
      const value = this.value;
      this.value = value + '\n';
    }
  });
  messageInput.oninput = function() {
    messageInput.style.height = '52px';
    messageInput.style.height = Math.min(messageInput.scrollHeight, 280) + 'px';
  };

  const messages = document.getElementById('messages');
  messages.addEventListener('scroll', function() {
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

  const vendorSelect = document.getElementById('vendorSelect');
  vendorSelect.addEventListener('change', function() {
    currentVendor = vendorSelect.value;
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
  if (sender === 'user') {
    messageRow.classList.add('message-row-right');

    let messageText = document.createElement('span');
    messageText.classList.add('message-body-right');
    activeDiv = messageText;
    messageRow.appendChild(messageText);
  } else {
    messageRow.classList.add('message-row');
    let messageSender = document.createElement('span');
    messageSender.classList.add('message-sender');
    messageSender.innerHTML =
      '<img width="30px" height="30px" src="https://cdn.jsdelivr.net/gh/samwang0723/project-allison@main/project_allison/static/' +
      sender +
      '.svg">';
    messageRow.appendChild(messageSender);

    let messageText = document.createElement('span');
    messageText.classList.add('message-body');
    activeDiv = messageText;
    messageRow.appendChild(messageText);
  }

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
        language === 'python' ||
        language === 'javascript' ||
        language === 'css' ||
        language === 'json' ||
        language === 'jsx' ||
        language === 'markdown' ||
        language === 'typescript' ||
        language === 'tsx'
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
    var imageClick = function() {
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

function toggleDarkMode() {
  document.body.classList.toggle('dark-mode');
}

function toggleLightMode() {
  document.body.classList.remove('dark-mode');
}

function uploadImageToImgur(file) {
  const clientId = '507bd7729a21e71'; // Replace with your Imgur client ID
  const formData = new FormData();
  formData.append('image', file);

  fetch('https://api.imgur.com/3/image', {
    method: 'POST',
    headers: {
      Authorization: 'Client-ID ' + clientId
    },
    body: formData
  })
    .then((response) => response.json())
    .then((data) => {
      if (data.success) {
        console.log('Image uploaded successfully:', data.data.link);
        currentImage = data.data.link;
        // You can now use the URL (data.data.link) as needed
        const thumbnailContainer =
          document.getElementById('thumbnailContainer');
        thumbnailContainer.innerHTML = `
          <img src="${parseThumbnail(data.data.link)}" class='thumbnail' alt='Thumbnail'>
          <button class='delete-btn' onclick='removeImage()'> X </button>
      `;
      } else {
        console.error('Image upload failed:', data);
      }
    })
    .catch((error) => {
      console.error('Error uploading image:', error);
    });
}

function removeImage() {
  const thumbnailContainer =
    document.getElementById('thumbnailContainer');
  thumbnailContainer.innerHTML = '';
  currentImage = '';
}

function parseThumbnail(filename) {
  // Find the last dot in the filename to separate the name and extension
  const lastDotIndex = filename.lastIndexOf('.');

  // If there's no dot, return the original filename
  if (lastDotIndex === -1) {
    return filename;
  }

  // Split the filename into name and extension
  const name = filename.substring(0, lastDotIndex);
  const extension = filename.substring(lastDotIndex);

  // Add 's' to the name
  const newName = name + 'l';

  // Reconstruct the filename
  const newFilename = newName + extension;

  return newFilename;
}

function storeMessage(owner, message) {
  if (!hasIndexDB) {
    return;
  }
  const request = indexedDB.open('artifical-chat', 1);

  request.onsuccess = function(event) {
    const db = event.target.result;
    const transaction = db.transaction(['messages'], 'readwrite');
    const objectStore = transaction.objectStore('messages');
    const addRequest = objectStore.add({ owner: owner, content: message, timestamp: new Date() });

    addRequest.onsuccess = function(event) {
      console.log('Message stored successfully');
    };

    addRequest.onerror = function(event) {
      console.error('Error storing message: ', event.target.errorCode);
    };
  };

  request.onerror = function(event) {
    console.error('Database error: ', event.target.errorCode);
  };
}


function loadMessages(callback) {
  if (!hasIndexDB) {
    return;
  }

  const request = indexedDB.open('artifical-chat', 1);

  request.onsuccess = function(event) {
    const db = event.target.result;
    const transaction = db.transaction(['messages'], 'readonly');
    const objectStore = transaction.objectStore('messages');
    const messages = [];

    objectStore.openCursor().onsuccess = function(event) {
      const cursor = event.target.result;
      if (cursor) {
        messages.push(cursor.value);
        cursor.continue();
      } else {
        callback(messages);
      }
    };

    objectStore.openCursor().onerror = function(event) {
      console.error('Error loading messages: ', event.target.errorCode);
    };
  };

  request.onerror = function(event) {
    console.error('Database error: ', event.target.errorCode);
  };
}

// Callback function to print out messages
function displayHistoricalMessages(messages) {
  messages.forEach(function(message) {
    addMessageRow(message.owner);
    formatMessage(message.content, true);
  });
}

function resetMessagesObjectStore(dbName = 'artifical-chat', storeName = 'messages') {
  if (!hasIndexDB) {
    return;
  }
  return new Promise((resolve, reject) => {
    const openRequest = indexedDB.open(dbName);

    openRequest.onerror = function(event) {
      console.error('Error opening database:', event.target.error);
      reject('Error opening database');
    };

    openRequest.onsuccess = function(event) {
      const db = event.target.result;
      const transaction = db.transaction([storeName], 'readwrite');
      const objectStore = transaction.objectStore(storeName);

      const clearRequest = objectStore.clear();

      clearRequest.onerror = function(event) {
        console.error('Error clearing object store:', event.target.error);
        reject('Error clearing object store');
      };

      clearRequest.onsuccess = function() {
        console.log('Object store cleared successfully');
        resolve('Object store cleared successfully');
      };
    };
  });
}
